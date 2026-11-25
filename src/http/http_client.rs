use crate::http::request::header_parser;
use crate::http::tls::setup_tls_connector;
use indicatif::{ProgressBar, ProgressStyle};
use serde_json::Value;
use std::fs::File;
use std::io::Write;
use std::io::{Error, ErrorKind};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{
    split, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, ReadBuf,
};
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;

pub struct HttpClient {
    pub host: String,
    pub path: String,
    pub stream: ClientStream,
}

#[derive(Debug)]
pub enum ClientStream {
    Tcp(TcpStream),
    Tls(Box<TlsStream<TcpStream>>),
}

impl AsyncRead for ClientStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            ClientStream::Tcp(tcp) => Pin::new(tcp).poll_read(cx, buf),
            ClientStream::Tls(tls) => Pin::new(tls).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for ClientStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        match self.get_mut() {
            ClientStream::Tcp(tcp) => Pin::new(tcp).poll_write(cx, buf),
            ClientStream::Tls(tls) => Pin::new(tls).poll_write(cx, buf),
        }
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        match self.get_mut() {
            ClientStream::Tcp(tcp) => Pin::new(tcp).poll_flush(cx),
            ClientStream::Tls(tls) => Pin::new(tls).poll_flush(cx),
        }
    }
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        match self.get_mut() {
            ClientStream::Tcp(tcp) => Pin::new(tcp).poll_flush(cx),
            ClientStream::Tls(tls) => Pin::new(tls).poll_flush(cx),
        }
    }
}

impl HttpClient {
    pub async fn open(url: &str) -> std::io::Result<Self> {
        let pared_url = Self::parse_url(url)?;
        let tcp_stream = TcpStream::connect(format!("{}:{}", pared_url.1, pared_url.2)).await?;
        let stream = if pared_url.2 == 443 {
            let tls_stream = setup_tls_connector(pared_url.1.clone(), tcp_stream).await;
            ClientStream::Tls(Box::from(tls_stream))
        } else {
            ClientStream::Tcp(tcp_stream)
        };
        Ok(HttpClient {
            host: pared_url.1,
            path: pared_url.3,
            stream,
        })
    }

    pub async fn send_request_json(&mut self, packet: &[u8]) -> std::io::Result<Option<Value>> {
        self.stream.write_all(packet).await?;
        let mut read_data = Vec::new();
        loop {
            let mut response = vec![0; 8192];
            let read = match self.stream.read(&mut response).await {
                Ok(read) => read,
                Err(error) if error.kind() == ErrorKind::UnexpectedEof => {
                    break;
                }
                Err(error) => return Err(error),
            };
            if read == 0 {
                break;
            }
            read_data.extend_from_slice(&response[..read]);
        }
        Ok(Self::parse_http_json_response(&read_data))
    }

    fn parse_http_json_response(response: &[u8]) -> Option<Value> {
        let header_end =
            if let Some(index) = response.windows(4).position(|window| window == b"\r\n\r\n") {
                (index, 4)
            } else {
                let index = response.windows(2).position(|window| window == b"\n\n")?;
                (index, 2)
            };

        let header = &response[..header_end.0];
        let body = &response[header_end.0 + header_end.1..];
        let status = String::from_utf8_lossy(header)
            .lines()
            .next()
            .unwrap_or("")
            .split_whitespace()
            .nth(1)
            .unwrap_or("")
            .trim()
            .to_string();
        if status != "200" {
            return None;
        }

        let headers = String::from_utf8_lossy(header).to_ascii_lowercase();
        let body = if headers.contains("transfer-encoding:") && headers.contains("chunked") {
            Self::decode_chunked_body(body)?
        } else {
            body.to_vec()
        };

        serde_json::from_slice::<Value>(&body).ok()
    }

    fn decode_chunked_body(mut body: &[u8]) -> Option<Vec<u8>> {
        let mut decoded = Vec::new();
        loop {
            let line_end = body.windows(2).position(|window| window == b"\r\n")?;
            let size_line = std::str::from_utf8(&body[..line_end]).ok()?;
            let size_hex = size_line.split(';').next()?.trim();
            let size = usize::from_str_radix(size_hex, 16).ok()?;
            body = &body[line_end + 2..];
            if size == 0 {
                return Some(decoded);
            }
            if body.len() < size {
                return None;
            }
            decoded.extend_from_slice(&body[..size]);
            body = &body[size..];
            if body.starts_with(b"\r\n") {
                body = &body[2..];
            } else if body.starts_with(b"\n") {
                body = &body[1..];
            } else {
                return None;
            }
        }
    }

    pub async fn download_file(&mut self, file_name: &str) {
        let request = format!(
            "GET /{} HTTP/1.1\r\n\
        accept-encoding: gzip, deflate, br, zstd\r\n\
        Host: {}\r\n\r\n",
            self.path, self.host
        );
        self.stream.write_all(request.as_bytes()).await.unwrap();
        let (socket_r, _) = split(&mut self.stream);
        let mut buf_reader = BufReader::with_capacity(8 * 1024, socket_r);
        Self::download_stream(&mut buf_reader, file_name).await
    }

    async fn download_stream<R>(buf_reader: &mut BufReader<R>, file_name: &str)
    where
        R: AsyncReadExt + Unpin,
    {
        //read status line
        if let Ok(Some(status_line)) = buf_reader.lines().next_line().await {
            if status_line.to_lowercase().contains("200 ok") {
                //headers
                let parsed_headers = header_parser(buf_reader).await;
                //read body
                let body_len = parsed_headers.get("Content-Length");
                if let Some(body_len) = body_len {
                    let body_len = body_len.parse::<usize>().unwrap();
                    let pb = ProgressBar::new(body_len as u64);
                    pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                        .unwrap()
                        .progress_chars("#>-"));
                    let mut file = File::create(file_name).unwrap();
                    let mut buffer = [0; 1024];
                    let mut read_buff = 0;
                    'body_loop: loop {
                        let n = buf_reader.read(&mut buffer).await.unwrap();
                        read_buff += n;
                        pb.set_position(read_buff as u64);
                        file.write_all(&buffer[..n]).unwrap();
                        if read_buff >= body_len {
                            break 'body_loop;
                        }
                    }
                    pb.finish_with_message("Download completed");
                }
            }
        }
    }

    fn parse_url(url: &str) -> std::io::Result<(String, String, i32, String)> {
        if let Some((scheme, rest)) = url.split_once("://") {
            let (host, path) = if rest.contains('/') {
                rest.split_once('/').unwrap()
            } else {
                (rest, "")
            };
            let port = if scheme == "https" { 443 } else { 80 };
            Ok((scheme.to_string(), host.to_string(), port, path.to_string()))
        } else {
            Err(Error::other("Error parsing input url"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::HttpClient;

    #[test]
    fn parses_plain_http_json_response() {
        let response = b"HTTP/1.1 200 OK\r\nContent-Length: 16\r\n\r\n{\"ip\":\"127.0.0.1\"}";
        let parsed = HttpClient::parse_http_json_response(response);
        assert_eq!(
            parsed.and_then(|value| value["ip"].as_str().map(str::to_string)),
            Some("127.0.0.1".to_string())
        );
    }

    #[test]
    fn parses_chunked_http_json_response() {
        let response =
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n2a\r\n{\"ip\":\"128.94.2.134\",\"org\":\"AS10796 Test\"}\r\n0\r\n\r\n";
        let parsed = HttpClient::parse_http_json_response(response);
        assert_eq!(
            parsed.and_then(|value| value["ip"].as_str().map(str::to_string)),
            Some("128.94.2.134".to_string())
        );
    }
}
