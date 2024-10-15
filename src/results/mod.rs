use std::net::IpAddr;
use std::str::FromStr;
use serde::{Deserialize, Serialize};

pub mod telemetry;
pub mod stats;

#[derive(Deserialize,Serialize, Debug,Clone)]
pub struct TelemetryData {
    pub ip_address : String,
    pub isp_info : String,
    pub extra : String,
    pub user_agent : String,
    pub lang : String,
    pub download : String,
    pub upload : String,
    pub ping : String,
    pub jitter : String,
    pub log : String,
    pub uuid : String,
    pub timestamp : i64,
}

pub fn redact_hostname(s: &mut String, replacement: &str) {
    let mut result = String::with_capacity(s.len());
    let mut idx = 0;
    while let Some(start_relative) = s[idx..].find("\"hostname\":\"") {
        let start = start_relative + idx;
        result.push_str(&s[idx..start]);
        result.push_str(replacement);
        let value_start = start + "\"hostname\":\"".len();
        if let Some(end_relative) = s[value_start..].find('\"') {
            let end = value_start + end_relative + 1;
            idx = end;
        } else {
            break;
        }
    }
    result.push_str(&s[idx..]);
    *s = result
}

pub fn redact_all_ips(s: &mut String, replacement: &str) {
    let mut result = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let mut idx = 0;
    while idx < len {
        let max_ip_length = 39; // IPV6 max len
        let remaining = len - idx;
        let max_len = if remaining < max_ip_length { remaining } else { max_ip_length };
        let mut replaced = false;
        for l in (2..=max_len).rev() {
            let end = idx + l;
            if end > len {
                continue;
            }
            let substr: String = chars[idx..end].iter().collect();
            if IpAddr::from_str(&substr).is_ok() {
                result.push_str(replacement);
                idx += l;
                replaced = true;
                break;
            }
        }
        if replaced {
            continue;
        }
        result.push(chars[idx]);
        idx += 1;
    }
    *s = result
}
