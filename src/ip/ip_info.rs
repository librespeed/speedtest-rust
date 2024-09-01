use std::net::{IpAddr, Ipv4Addr};
use log::warn;
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::config::SERVER_CONFIG;
use crate::http::http_client::HttpClient;
use crate::ip::mmdb::mmdb_reader::MMDBReader;
use crate::ip::mmdb::mmdb_record::MMDBResult;

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
pub struct IPInfo {
    pub processedString : String,
    pub rawIspInfo : RawISPInfo
}

#[derive(Serialize, Deserialize)]
pub struct RawISPInfo {
    pub ip : String,
    pub hostname : String,
    pub city : String,
    pub region : String,
    pub country : String,
    #[serde(rename = "loc")]
    pub location : String,
    #[serde(rename = "org")]
    pub organization : String,
    pub postal : String,
    pub timezone : String,
    pub readme : Option<String>
}

impl Default for IPInfo {
    fn default() -> Self {
        IPInfo {
            processedString : "".to_string(),
            rawIspInfo : RawISPInfo {
                ip : "".to_string(),
                hostname : "".to_string(),
                city : "".to_string(),
                region : "".to_string(),
                country : "".to_string(),
                location : "".to_string(),
                organization : "".to_string(),
                postal : "".to_string(),
                timezone : "".to_string(),
                readme : None
            }
        }
    }
}

impl IPInfo {
    pub async fn fetch_information(raw_ip : &str,get_isp : bool) -> String {
        let mut ip_info_model = IPInfo::default();

        if !get_isp {
            ip_info_model.processedString = raw_ip.to_string();
            let json_string = json!(ip_info_model);
            return json_string.to_string()
        }

        //local ip, no need to get more information
        if let Some(local_ip_info) = Self::get_local_or_private_info(raw_ip) {
            ip_info_model.processedString = format!("{} - {}",raw_ip,local_ip_info);
            let json_string = json!(ip_info_model);
            return json_string.to_string()
        }

        //get isp info from api
        if let Some(isp_info) = Self::get_isp_info_from_api(raw_ip).await {
            ip_info_model.processedString = isp_info;
            let json_string = json!(ip_info_model);
            return json_string.to_string()
        }

        //get isp info from db
        if let Some(isp_info) = Self::get_isp_info_from_db(raw_ip) {
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

    fn get_local_or_private_info (ip : &str) -> Option<String> {
        if "::1" == ip {
            return Some("localhost IPv6 access".to_string())
        }
        if ip.starts_with("fe80:") {
            return Some("link-local IPv6 access".to_string())
        }
        if Self::is_ula_ipv6(ip) {
            return Some("ULA IPv6 access".to_string())
        }
        if ip.starts_with("127.") {
            return Some("localhost IPv4 access".to_string())
        }
        if Self::is_private_ipv4(ip) {
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

    async fn get_isp_info_from_api(ip : &str) -> Option<String> {
        let config = SERVER_CONFIG.get()?;
        let ip_info_token = config.ipinfo_api_key.clone();
        if ip_info_token.is_empty() {
            return None
        }
        if let Ok(mut client) = HttpClient::open("https://ipinfo.io").await {
            let request = format!(
                "GET /{}/json?token={} HTTP/1.1\r\n\
                Host: ipinfo.io\r\n\r\n",
                ip,
                ip_info_token
            );
            if let Ok(Some(res_body)) = client.send_request_json(request.as_bytes()).await {
                let isp = if let Some(org) = res_body.get("org") {
                    Some(org.as_str()?)
                } else {
                    let asn_name = &res_body["asn"]["name"];
                    if !asn_name.is_null() {
                        Some(asn_name.as_str()?)
                    } else {
                        None
                    }
                };
                isp.as_ref()?;
                let output = format!("{} - {}, {}",ip,isp?,res_body.get("country")?.as_str()?);
                Some(output)
            } else {
                None
            }
        } else {
            warn!("Http client error.");
            None
        }
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

}