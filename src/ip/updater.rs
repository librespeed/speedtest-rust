use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use indicatif::{ProgressBar, ProgressStyle};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, split};
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use tokio_rustls::rustls::{ClientConfig, RootCertStore};
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::TlsConnector;
use crate::http::request::header_parser;
use crate::http::simple_parse_url;

pub fn update_ipdb(url : &str,file_name : &str) {
    let runtime = Runtime::new().unwrap();
    runtime.block_on(async {
        let parsed_url = simple_parse_url(url);
        let mut root_cert_store = RootCertStore::empty();
        root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let config = ClientConfig::builder().with_root_certificates(root_cert_store).with_no_client_auth();
        //open tls connector
        let dns_name = ServerName::try_from(parsed_url.1.clone()).unwrap();
        let connector = TlsConnector::from(Arc::new(config));
        //open stream
        let socket = TcpStream::connect(format!("{}:{}",&parsed_url.1,&parsed_url.2)).await.unwrap();
        let mut socket = connector.connect(dns_name, socket).await.unwrap();
        let request =
            format!("GET /{} HTTP/1.1\r\n\
        accept-encoding: gzip, deflate, br, zstd\r\n\
        Host: {}\r\n\r\n",parsed_url.3,parsed_url.1);
        socket.write(request.as_bytes()).await.unwrap();
        let (socket_r, _) = split(socket);
        let mut buff_reader = BufReader::with_capacity(8 * 1024, socket_r);
        read_resp(&mut buff_reader,file_name).await
    });
}

async fn read_resp<R>(buf_reader: &mut BufReader<R>,file_name : &str)
    where
        R : AsyncReadExt + Unpin
{
    'root_loop:loop {
        //read status line
        if let Ok(Some(status_line)) = buf_reader.lines().next_line().await {
            if !status_line.to_lowercase().contains("200 ok") {
                break 'root_loop;
            }
        } else {
            break 'root_loop;
        }
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
            pb.finish_with_message("Download IPdb completed");
            break 'root_loop;
        } else {
            break 'root_loop;
        }
    }
}