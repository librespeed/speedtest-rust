use std::fs::File;
use std::io::{Error, ErrorKind};
use std::pin::Pin;
use std::task::{Context, Poll};
use serde_json::Value;
use tokio::io::{split, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, ReadBuf};
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use std::io::Write;
use indicatif::{ProgressBar, ProgressStyle};
use crate::http::request::header_parser;
use crate::http::tls::setup_tls_connector;

pub struct HttpClient {
    pub host : String,
    pub path : String,
    pub stream : ClientStream
}

#[derive(Debug)]
pub enum ClientStream {
    Tcp(TcpStream),
    Tls(Box<TlsStream<TcpStream>>),
}

impl AsyncRead for ClientStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            ClientStream::Tcp(tcp) => Pin::new(tcp).poll_read(cx, buf),
            ClientStream::Tls(tls) => Pin::new(tls).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for ClientStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, Error>> {
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

    pub async fn open(url : &str) -> std::io::Result<Self> {
        let pared_url = Self::parse_url(url)?;
        let tcp_stream = TcpStream::connect(format!("{}:{}",pared_url.1,pared_url.2)).await?;
        let stream = if pared_url.2 == 443 {
            let tls_stream = setup_tls_connector(pared_url.1.clone(),tcp_stream).await;
            ClientStream::Tls(Box::from(tls_stream))
        } else {
            ClientStream::Tcp(tcp_stream)
        };
        Ok(HttpClient {
            host : pared_url.1,
            path : pared_url.3,
            stream
        })
    }

    pub async fn send_request_json(&mut self,packet : &[u8]) -> std::io::Result<Option<Value>> {
        self.stream.write_all(packet).await?;
        let mut read_data = Vec::new();
        loop {
            let mut response = vec![0; 128];
            let read = self.stream.read(&mut response).await?;
            read_data.extend(response);
            if read < 128 { //EOF
                break;
            }
        }
        let parser = String::from_utf8_lossy(&read_data);
        let response = parser.trim_matches(char::from(0));
        if response.starts_with("HTTP/1.1 200") {
            let mut split_body = response.splitn(2,"\r\n\r\n");
            let resp_body = split_body.nth(1).unwrap_or("");
            if let Ok(parsed_json_body) = serde_json::from_str::<Value>(resp_body) {
                Ok(Some(parsed_json_body))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub async fn download_file(&mut self,file_name : &str) {
        let request = format!("GET /{} HTTP/1.1\r\n\
        accept-encoding: gzip, deflate, br, zstd\r\n\
        Host: {}\r\n\r\n",self.path,self.host);
        self.stream.write_all(request.as_bytes()).await.unwrap();
        let (socket_r, _) = split(&mut self.stream);
        let mut buf_reader = BufReader::with_capacity(8 * 1024, socket_r);
        Self::download_stream(&mut buf_reader,file_name).await
    }

    async fn download_stream<R>(buf_reader: &mut BufReader<R>,file_name : &str)
    where
        R : AsyncReadExt + Unpin
    {
        //read status line
        if let Ok(Some(status_line)) = buf_reader.lines().next_line().await {
            if status_line.to_lowercase().contains("200 ok") {
                //headers
                let parsed_headers = header_parser(buf_reader).await;
                //read body
                let body_len = parsed_headers.get("Content-Length");
                if body_len.is_some() {
                    let body_len = body_len.unwrap().parse::<usize>().unwrap();
                    let pb = ProgressBar::new(body_len as u64);
                    pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                        .unwrap()
                        .progress_chars("#>-"));
                    let mut file = File::create(file_name).unwrap();
                    let mut buffer = [0; 1024];
                    let mut read_buff = 0;
                    'body_loop:loop {
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

    fn parse_url(url: &str) -> std::io::Result<(String,String,i32,String)> {
        if let Some((scheme, rest)) = url.split_once("://") {
            let (host, path) = if rest.contains('/') {
                rest.split_once('/').unwrap()
            } else {
                (rest,"")
            };
            let port = if scheme == "https" { 443 } else { 80 };
            Ok((scheme.to_string(),host.to_string(),port,path.to_string()))
        } else {
            Err(Error::new(ErrorKind::Other,"Error parsing input url"))
        }
    }

}