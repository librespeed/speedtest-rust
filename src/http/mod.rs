use std::collections::HashMap;
use tokio::net::TcpStream;
use crate::http::request::Request;

pub mod http_server;
mod routes;
pub mod request;
pub mod response;
pub mod cookie;

#[derive(Debug)]
pub enum Method {
    Get,
    Post,
}

pub trait MethodStr {
    fn to_method(&self) -> Method;
}

impl MethodStr for str {
    fn to_method(&self) -> Method {
        match self {
            "GET" => Method::Get,
            "POST" => Method::Post,
            _ => Method::Get
        }
    }
}

pub async fn find_remote_ip_addr (conn: &mut TcpStream) -> String {
    let client_addr = conn.peer_addr().unwrap();
    client_addr.ip().to_string().replace("::ffff:","")
}

pub fn check_route_for_index_page(req : &Request) -> bool {
    let path = req.path.clone();
    let sep_count = path.chars().filter(|c| *c == '/').count();
    matches!(req.method, Method::Get) && (sep_count == 1)
}

pub fn get_chunk_count (query_params : &HashMap<String,String>) -> i32 {
    let mut chunks = 4;
    if let Some(ck_size) = query_params.get("ckSize") {
        if let Ok(parsed_ck_size) = ck_size.parse::<i32>() {
            if parsed_ck_size > 1024 {
                chunks = 1024
            } else {
                chunks = parsed_ck_size
            }
        }
    }
    chunks *= 2;
    chunks
}

#[macro_export]
macro_rules! make_route {
    ($a:expr) => {
        {
            let base_url = SERVER_CONFIG.get().unwrap().base_url.clone();
            format!("{}{}",base_url,$a)
        }
    };
}