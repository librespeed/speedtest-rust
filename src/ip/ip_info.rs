use std::net::{IpAddr, Ipv4Addr};
use log::warn;
use serde_json::json;
use crate::ip::IPInfo;
use crate::ip::mmdb::mmdb_reader::MMDBReader;
use crate::ip::mmdb::mmdb_record::MMDBResult;

pub fn get_ip_info (raw_ip : &str,get_isp : bool) -> String {
    let mut ip_info_model = IPInfo::empty();

    //no need to get isp info
    if !get_isp {
        ip_info_model.processedString = raw_ip.to_string();
        let json_string = json!(ip_info_model);
        return json_string.to_string()
    }

    //local ip, no need to get more information
    if let Some(local_ip_info) = get_local_or_private_info(raw_ip) {
        ip_info_model.processedString = format!("{} - {}",raw_ip,local_ip_info);
        let json_string = json!(ip_info_model);
        return json_string.to_string()
    }

    //get isp info from db or api
    if let Some(isp_info) = get_isp_info_from_db(raw_ip) {
        ip_info_model.processedString = format!("{} - {}, {}",raw_ip,isp_info.as_name,isp_info.country_name);
        ip_info_model.rawIspInfo.ip = raw_ip.to_string();
        ip_info_model.rawIspInfo.country = isp_info.country;
        ip_info_model.rawIspInfo.organization = format!("{} {}",isp_info.asn,isp_info.as_name);
        let json_string = json!(ip_info_model);
        json_string.to_string()
    } else {

        //failed to get isp and send only ip
        ip_info_model.processedString = raw_ip.to_string();
        let json_string = json!(ip_info_model);
        json_string.to_string()
    }
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

fn get_isp_info_from_db(ip : &str) -> Option<MMDBResult> {
    if let Some(mut ipdb_reader) = MMDBReader::from("country_asn.mmdb") {
        return ipdb_reader.lookup(ip)
    }
    warn!("Unable to open country asn database file");
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