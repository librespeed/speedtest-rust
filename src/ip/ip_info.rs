use std::net::{IpAddr, Ipv4Addr};
use serde_json::json;
use crate::ip::{IPInfo};

pub fn get_ip_info (raw_ip : &str,get_isp : bool) -> String {
    let mut ip_info_model = IPInfo::empty();

    if let Some(local_ip_info) = get_local_or_private_info(raw_ip) {
        ip_info_model.processedString = format!("{} - {}",raw_ip,local_ip_info);
        let json_string = json!(ip_info_model);
        return json_string.to_string()
    }

    if !get_isp {
        // no need to get isp info
        ip_info_model.processedString = raw_ip.to_string();
        let json_string = json!(ip_info_model);
        return json_string.to_string()
    }

    //get isp info


    let json_string = json!(ip_info_model);
    json_string.to_string()
}

pub fn get_local_or_private_info (ip : &str) -> Option<String> {
    if "::1" == ip {
        return Some("localhost IPv6 access".to_string())
    }
    if ip.starts_with("fe80:") {
        return Some("link-local IPv6 access".to_string())
    }
    if is_ula_ipv6(ip) {
        return Some("ULA IPv6 access".to_string())
    }
    if ip.starts_with("127.") {
        return Some("localhost IPv4 access".to_string())
    }
    if is_private_ipv4(ip) {
        return Some("private IPv4 access".to_string())
    }
    if ip.starts_with("169.254.") {
        return Some("link-local IPv4 access".to_string())
    }
    None
}

fn is_private_ipv4(ip: &str) -> bool {
    if let Ok(ip_addr) = ip.parse::<Ipv4Addr>() {
        ip_addr.is_private()
    } else {
        false
    }
}

fn is_ula_ipv6(ip: &str) -> bool {
    match ip.parse::<IpAddr>() {
        Ok(IpAddr::V6(v6_addr)) => (v6_addr.octets()[0] & 0xc0) == 0xc0,
        _ => false
    }
}