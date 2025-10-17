use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use log::trace;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use case_insensitive_hashmap::CaseInsensitiveHashMap as CIHashMap;
use crate::config::GARBAGE_DATA;
use crate::http::{Method, MethodStr};
use crate::http::response::Response;

#[derive(Debug)]
pub struct Request {
    pub path: String,
    pub method: Method,
    pub remote_addr : String,
    pub query_params: HashMap<String, String>,
    pub headers: CIHashMap<String>,
    pub form_data : HashMap<String, String>
}

#[derive(Debug)]
enum BodyType {
    Fixed,
    Chunked,
    Form,
    FormUrlEncoded
}

pub async fn handle_socket<R,W,F>(remote_addr : &str,buf_reader: &mut BufReader<R>,buf_writer : &mut BufWriter<W>,result : F)
    where
R: AsyncReadExt + Unpin,
W: AsyncWriteExt + Unpin,
F: Send + Sync + Fn(Request) -> Pin<Box<dyn Future<Output = Response> + Send>>
{
    'root_loop:loop {
        //read status line
        let parsed_status = {
            if let Ok(Some(status_line)) = buf_reader.lines().next_line().await {
                let status_lower = status_line.to_lowercase();
                if check_is_status_line(status_lower) {
                    parse_request_status_line(status_line)
                } else {
                    break 'root_loop;
                }
            } else {
                break 'root_loop;
            }
        };
        //read headers
        let parsed_headers = header_parser(buf_reader).await;
        //read body content
        let body_form_data = {
            let (body_type,body_size) = check_has_body(&parsed_headers);
            match body_type {
                Some(body_type) => {
                    match body_type {
                        BodyType::Fixed => {
                            match body_size {
                                Some(body_size) => {
                                    let mut buffer = [0; 1024];
                                    let mut len: usize = 0;
                                    loop {
                                        let bytes_read = buf_reader.read_exact(&mut buffer).await;
                                        match bytes_read {
                                            Ok(0) => {
                                                buffer.fill(0);
                                                break;
                                            }
                                            Ok(_) => {
                                                len += buffer.len();
                                                if len >= body_size as usize {
                                                    buffer.fill(0);
                                                    break;
                                                }
                                            }
                                            Err(_) => {
                                                break;
                                            }
                                        }
                                    }
                                    None
                                }
                                None => {
                                    None
                                }
                            }
                        }
                        BodyType::Chunked => {
                            let mut buffer = [0; 1024];
                            loop {
                                let bytes_read = buf_reader.read_exact(&mut buffer).await;
                                match bytes_read {
                                    Ok(0) => {
                                        buffer.fill(0);
                                        break;
                                    }
                                    Ok(_) => {
                                        buffer.fill(0);
                                    }
                                    Err(_) => {
                                        break;
                                    }
                                }
                            }
                            None
                        }
                        BodyType::Form => {
                            let form_boundary = get_content_boundary(parsed_headers.get("Content-Type").unwrap());
                            match form_boundary {
                                Some(form_boundary) => {
                                    match body_size {
                                        Some(body_size) => {
                                            let mut body = Vec::with_capacity(body_size as usize);
                                            buf_reader.take(body_size).read_to_end(&mut body).await.unwrap();
                                            let form_data = parse_form_data(&form_boundary,&body);
                                            body.fill(0);
                                            Some(form_data)
                                        }
                                        None => {
                                            None
                                        }
                                    }
                                }
                                None => {
                                    None
                                }
                            }
                        }
                        BodyType::FormUrlEncoded => {
                            match body_size {
                                Some(body_size) => {
                                    let mut body = Vec::with_capacity(body_size as usize);
                                    buf_reader.take(body_size).read_to_end(&mut body).await.unwrap();
                                    let form_data = parse_form_url_encoded(&body);
                                    body.fill(0);
                                    Some(form_data)
                                }
                                None => {
                                    None
                                }
                            }
                        }
                    }
                }
                None => {
                    None
                }
            }
        };
        //trust proxy
        let remote_addr = trust_addr_proxy(&parsed_headers,remote_addr);
        //gen request
        let response = result(Request {
            path: parsed_status.1,
            method: parsed_status.0,
            remote_addr,
            query_params: parsed_status.2,
            headers: parsed_headers,
            form_data : body_form_data.clone().unwrap_or(HashMap::new())
        }).await;
        if let Err(e) = buf_writer.write_all(&response.data).await {
            trace!("Error socket write : {e}")
        }
        if response.chunk_count > 0 {
            for _ in 0..response.chunk_count {
                if let Err(e) = buf_writer.write_all(GARBAGE_DATA.get().unwrap()).await {
                    trace!("Error socket write chunk : {e}")
                }
            }
            if let Err(e) = buf_writer.write_all(b"0\r\n\r\n").await {
                trace!("Error socket write eof : {e}")
            }
        }
        if let Err(e) = buf_writer.flush().await {
            trace!("Error socket flush : {e}")
        }
    }
}

//allow http 1.* & POST, GET, OPTIONS methods
fn check_is_status_line (line : String) -> bool {
    line.contains("http/1.") && (line.starts_with("get") || line.starts_with("options") || line.starts_with("post"))
}

#[allow(dead_code)]
fn hex_string_to_int(hex_string: &str) -> Option<u64> {
    u64::from_str_radix(hex_string, 16).ok()
}

pub async fn header_parser<R>(buf_reader: &mut BufReader<R>) -> CIHashMap<String>
where
    R: AsyncReadExt + Unpin
{
    let mut headers_out = CIHashMap::new();
    'header_loop:loop {
        if let Ok(Some(header_line)) = buf_reader.lines().next_line().await {
            if header_line.is_empty() {
                break 'header_loop;
            } else {
                let mut header_parts = header_line.splitn(2, ':');
                if let (Some(header_key),Some(header_val)) = (header_parts.next(),header_parts.next()) {
                    headers_out.insert(header_key.trim().to_string(),header_val.trim().to_string());
                }
            }
        } else {
            break 'header_loop;
        }
    }
    headers_out
}

fn check_has_body(headers : &CIHashMap<String>) -> (Option<BodyType>,Option<u64>) {
    let content_type_form = if let Some(content_type) = headers.get("Content-Type") {
        if content_type.starts_with("multipart/form-data;") {
            Some(BodyType::Form)
        } else if content_type.starts_with("application/x-www-form-urlencoded") {
            Some(BodyType::FormUrlEncoded)
        } else {
            None
        }
    } else {
        None
    };
    //check fixed body
    if let Some(content_len) = headers.get("Content-Length") {
        let content_len = content_len.parse::<u64>().unwrap_or(0);
        if content_len > 0 {
            let body_type = if let Some(content_type_form) = content_type_form {
                content_type_form
            } else {
                BodyType::Fixed
            };
            return (Some(body_type),Some(content_len))
        };
    };
    //check chunked body
    if let Some(transfer_encoding) = headers.get("Transfer-Encoding") {
        if transfer_encoding == "chunked" {
            return (Some(BodyType::Chunked),Some(0))
        }
    }
    (None,None)
}

fn parse_request_status_line (line : String) -> (Method,String,HashMap<String, String>) {
    let mut split_status = line.split(' ');
    let method_str = split_status.next().unwrap();
    let raw_path = split_status.next().unwrap();
    let (path,query_params) = parse_raw_path(raw_path);
    (method_str.to_method(),path.to_string(),query_params)
}

fn parse_raw_path(raw_path: &str) -> (&str, HashMap<String, String>) {
    let mut real_path = raw_path;
    let mut query_params = HashMap::new();
    if raw_path.contains('?') {
        let split_raw_path = raw_path.split('?');
        let vec_path = split_raw_path.collect::<Vec<&str>>();
        real_path = clear_path_end_slash(vec_path[0]);
        let raw_query_params = vec_path[1];
        let split_raw_query_params = raw_query_params.split('&');
        for part in split_raw_query_params {
            let mut split_part = part.split('=');
            if let (Some(query_key),Some(query_val)) = (split_part.next(),split_part.next())  {
                query_params.insert(query_key.to_string(), query_val.to_string());
            }
        }
    }
    (real_path, query_params)
}

fn clear_path_end_slash(input: &str) -> &str {
    if let Some(strip) = input.strip_suffix('/') {
        strip
    } else {
        input
    }
}

fn trust_addr_proxy(headers : &CIHashMap<String>,remote_addr : &str) -> String {
    if let Some(remote_ip) = headers.get("X-Real-IP") {
        remote_ip.to_string()
    } else {
        remote_addr.to_string()
    }
}

//form-data-parser
fn get_content_boundary(content_type : &str) -> Option<String> {
    let parts = content_type.split(';');
    let mut boundary = None;
    for part in parts {
        let part = part.trim();
        if part.starts_with("boundary=") {
            let mut boundary_str = part.splitn(2,'=');
            let boundary_result = boundary_str.nth(1);
            match boundary_result {
                None => {},
                Some(boundary_result) => {
                    let mut boundary_p = "--".to_string();
                    boundary_p.push_str(boundary_result);
                    boundary = Some(boundary_p)
                }
            }
            break;
        }
    }
    boundary
}

fn parse_form_data(boundary : &str,body : &[u8]) -> HashMap<String,String> {
    let body_str = String::from_utf8_lossy(body);
    let mut form_data = HashMap::new();
    let form_parts = body_str.split(boundary);
    for form_part in form_parts {
        let form_part = form_part.trim();
        if !form_part.is_empty() && form_part != "--" {
            let mut body_parts = form_part.splitn(2,"\r\n");  //Content-Disposition: form-data; name="key"\r\nvalue
            if let (Some(disposition),Some(value)) = (body_parts.next(),body_parts.next()) {
                let value = value.replace("\r\n","");
                //parse key
                let mut split_disposition = disposition.splitn(2,';');
                if let Some(name_part) = split_disposition.nth(1) {
                    if let Some(key) = name_part.split_once('=').map(|x|x.1) {
                        let key = key.replace('"',"");
                        form_data.insert(key,value);
                    }
                }
            }
        }
    }
    form_data
}

fn parse_form_url_encoded(body : &[u8]) -> HashMap<String,String> {
    let body_str = String::from_utf8_lossy(body);
    let split_parts = body_str.split('&');
    let mut form_data = HashMap::new();
    for part in split_parts {
        let mut split_key_value = part.splitn(2,'=');
        if let (Some(key),Some(value)) = (split_key_value.next(),split_key_value.next()) {
            form_data.insert(key.to_string(),value.to_string());
        }
    };
    form_data
}