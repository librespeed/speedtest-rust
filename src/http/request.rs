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
        //honor `Expect: 100-continue` (RFC 9110 §10.1.1) by sending the interim 100 response
        //*before* reading the body. Without it, clients that use Expect (curl and many HTTP
        //libraries) wait out their continue-timeout (~1s for curl) on every upload request.
        if let Some(expect) = parsed_headers.get("Expect") {
            if expect.eq_ignore_ascii_case("100-continue")
                && buf_writer.write_all(b"HTTP/1.1 100 Continue\r\n\r\n").await.is_ok()
            {
                let _ = buf_writer.flush().await;
            }
        }
        //read body content
        let body_form_data = {
            let (body_type,body_size) = check_has_body(&parsed_headers);
            match body_type {
                Some(body_type) => {
                    match body_type {
                        BodyType::Fixed => {
                            match body_size {
                                Some(body_size) => {
                                    // Read and discard exactly `body_size` bytes. The previous loop
                                    // used `read_exact` into a fixed 1024-byte buffer and added
                                    // `buffer.len()` (always 1024) per iteration, so a Content-Length
                                    // that is not a multiple of 1024 left a trailing partial chunk that
                                    // `read_exact` would block on indefinitely (until the peer
                                    // disconnects), hanging the upload. Reading with `read` and counting
                                    // the *actual* bytes returned makes any Content-Length safe and
                                    // removes the need for client-side payload padding. The larger
                                    // buffer also cuts the per-1KB read-syscall overhead.
                                    let mut buffer = [0u8; 65536];
                                    let mut remaining = body_size as usize;
                                    while remaining > 0 {
                                        let want = remaining.min(buffer.len());
                                        match buf_reader.read(&mut buffer[..want]).await {
                                            Ok(0) => break,          // peer closed before sending it all
                                            Ok(n) => remaining -= n, // count what we actually read
                                            Err(_) => break,
                                        }
                                    }
                                    buffer.fill(0);
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
    headers.get("X-Real-IP")
        .map(|s| s.as_str())
        .or_else(|| {
            headers.get("X-Forwarded-For").and_then(|s| {
                s.split(',').next().map(|ip| ip.trim())
            })
        }).unwrap_or(remote_addr).to_string()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    // Minimal router stub: every request gets a 200, so the tests exercise body consumption
    // and the Expect handshake in `handle_socket` rather than the real routes.
    fn ok_router(_req: Request) -> Pin<Box<dyn Future<Output = Response> + Send>> {
        Box::pin(async { Response::res_200("") })
    }

    fn count_occurrences(haystack: &[u8], needle: &[u8]) -> usize {
        if needle.is_empty() || haystack.len() < needle.len() { return 0; }
        let mut count = 0;
        let mut i = 0;
        while i + needle.len() <= haystack.len() {
            if &haystack[i..i + needle.len()] == needle {
                count += 1;
                i += needle.len();
            } else {
                i += 1;
            }
        }
        count
    }

    async fn drive(raw: Vec<u8>) -> Vec<u8> {
        let mut reader = BufReader::new(Cursor::new(raw));
        let mut writer = BufWriter::new(Vec::new());
        handle_socket("127.0.0.1", &mut reader, &mut writer, ok_router).await;
        writer.flush().await.unwrap();
        writer.into_inner()
    }

    /// A Content-Length that is NOT a multiple of the read-buffer size must consume *exactly*
    /// `Content-Length` body bytes — not over-read into a following pipelined request. The old
    /// `read_exact([0; 1024])` loop demanded a full final 1024-byte chunk, swallowing bytes of the
    /// next request (and, against a live peer that keeps the socket open, blocking forever — the
    /// upload "hang" WiFiMaster/ops worked around with 1024/2048 payload padding).
    #[tokio::test]
    async fn fixed_body_is_not_over_read_into_a_pipelined_request() {
        let mut raw = Vec::new();
        raw.extend_from_slice(b"POST /empty HTTP/1.1\r\nContent-Length: 1500\r\n\r\n");
        raw.extend_from_slice(&vec![b'x'; 1500]); // 1500 % 1024 == 476
        raw.extend_from_slice(b"GET /empty HTTP/1.1\r\n\r\n");
        let out = drive(raw).await;
        assert_eq!(
            count_occurrences(&out, b"HTTP/1.1 200"), 2,
            "both the POST and the pipelined GET must be answered; the body must stop at Content-Length"
        );
    }

    /// `Expect: 100-continue` must get an interim `100 Continue` so the client sends its body
    /// immediately instead of waiting out its continue-timeout (~1s for curl) on every upload.
    #[tokio::test]
    async fn expect_100_continue_receives_interim_response() {
        let mut raw = Vec::new();
        raw.extend_from_slice(b"POST /empty HTTP/1.1\r\nExpect: 100-continue\r\nContent-Length: 2048\r\n\r\n");
        raw.extend_from_slice(&vec![b'x'; 2048]);
        let out = drive(raw).await;
        assert!(
            count_occurrences(&out, b"100 Continue") >= 1,
            "server must send an interim `100 Continue` for `Expect: 100-continue`"
        );
        assert!(
            count_occurrences(&out, b"HTTP/1.1 200") >= 1,
            "the final 200 response must still follow the interim 100"
        );
    }
}