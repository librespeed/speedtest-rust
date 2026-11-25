use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use log::warn;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::config::SERVER_CONFIG;
use crate::http::http_client::HttpClient;
use crate::ip::mmdb::mmdb_reader::{CityMMDBReader, MMDBReader};
use crate::ip::mmdb::mmdb_record::{MMDBCityResult, MMDBResult};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct IPInfo {
    #[serde(default)]
    pub processedString: String,
    #[serde(default, deserialize_with = "deserialize_raw_isp_info")]
    pub rawIspInfo: RawISPInfo,
    #[serde(default, deserialize_with = "deserialize_location_info")]
    pub location: LocationInfo,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RawISPInfo {
    #[serde(default)]
    pub ip: String,
    #[serde(default)]
    pub hostname: String,
    #[serde(default)]
    pub city: String,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub country: String,
    #[serde(default, rename = "loc")]
    pub location: String,
    #[serde(default, rename = "org")]
    pub organization: String,
    #[serde(default)]
    pub postal: String,
    #[serde(default)]
    pub timezone: String,
    #[serde(default)]
    pub readme: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LocationInfo {
    #[serde(default)]
    pub ip: String,
    #[serde(default)]
    pub city: String,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub country: String,
    #[serde(default, rename = "countryName", alias = "country_name")]
    pub country_name: String,
    #[serde(default)]
    pub loc: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub org: String,
    #[serde(default)]
    pub organization: String,
    #[serde(default)]
    pub isp: String,
    #[serde(default)]
    pub postal: String,
    #[serde(default)]
    pub timezone: String,
    #[serde(default)]
    pub asn: String,
    #[serde(default)]
    pub as_name: String,
    #[serde(default)]
    pub as_domain: String,
    #[serde(default)]
    pub continent: String,
    #[serde(default)]
    pub continent_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
    #[serde(default)]
    pub source: String,
}

fn deserialize_raw_isp_info<'de, D>(deserializer: D) -> Result<RawISPInfo, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    if let Ok(info) = serde_json::from_value::<RawISPInfo>(value) {
        Ok(info)
    } else {
        Ok(RawISPInfo::default())
    }
}

fn deserialize_location_info<'de, D>(deserializer: D) -> Result<LocationInfo, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    if let Ok(info) = serde_json::from_value::<LocationInfo>(value) {
        Ok(info)
    } else {
        Ok(LocationInfo::default())
    }
}

static COUNTRY_MMDB_CACHE: OnceLock<Mutex<Option<(String, MMDBReader)>>> = OnceLock::new();
static CITY_MMDB_CACHE: OnceLock<Mutex<Option<(String, CityMMDBReader)>>> = OnceLock::new();

impl IPInfo {
    pub async fn fetch_information(raw_ip: &str, get_isp: bool) -> String {
        let mut ip_info_model = IPInfo::default();
        ip_info_model.rawIspInfo.ip = raw_ip.to_string();
        ip_info_model.location.ip = raw_ip.to_string();

        if !get_isp {
            ip_info_model.processedString = raw_ip.to_string();
            ip_info_model.location.source = "none".to_string();
            return json!(ip_info_model).to_string();
        }

        if let Some(local_ip_info) = Self::get_local_or_private_info(raw_ip) {
            ip_info_model.processedString = format!("{raw_ip} - {local_ip_info}");
            ip_info_model.location.source = "local".to_string();
            return json!(ip_info_model).to_string();
        }

        if let Some(mut api_info) = Self::get_ipinfo_information(raw_ip).await {
            let offline_contributed = Self::merge_offline_location(raw_ip, &mut api_info);
            api_info.location.source = if offline_contributed {
                "ipinfo+mmdb".to_string()
            } else {
                "ipinfo".to_string()
            };
            api_info.processedString = Self::build_processed_string(
                raw_ip,
                &api_info.rawIspInfo.organization,
                &api_info.rawIspInfo.country,
                &api_info.rawIspInfo.city,
                &api_info.rawIspInfo.region,
                &api_info.location.country_name,
            );
            return json!(api_info).to_string();
        }

        if let Some(offline_info) = Self::get_offline_information(raw_ip) {
            return json!(offline_info).to_string();
        }

        ip_info_model.processedString = raw_ip.to_string();
        ip_info_model.location.source = "none".to_string();
        json!(ip_info_model).to_string()
    }

    pub fn parse_ipinfo_response(raw_ip: &str, res_body: &Value) -> IPInfo {
        let mut ip_info = IPInfo::default();

        ip_info.rawIspInfo.ip =
            first_string(res_body, &[&["ip"]]).unwrap_or_else(|| raw_ip.to_string());
        ip_info.location.ip = ip_info.rawIspInfo.ip.clone();
        ip_info.rawIspInfo.hostname = first_string(res_body, &[&["hostname"]]).unwrap_or_default();
        ip_info.rawIspInfo.city = first_string(res_body, &[&["city"]]).unwrap_or_default();
        ip_info.rawIspInfo.region = first_string(res_body, &[&["region"]]).unwrap_or_default();
        ip_info.rawIspInfo.country =
            normalize_country_code(&first_string(res_body, &[&["country"]]).unwrap_or_default());
        let api_location = first_string(res_body, &[&["loc"]]).unwrap_or_default();
        ip_info.rawIspInfo.location = if parse_coordinates(&api_location).is_some() {
            api_location
        } else {
            String::new()
        };
        ip_info.rawIspInfo.postal = first_string(res_body, &[&["postal"]]).unwrap_or_default();
        ip_info.rawIspInfo.timezone = first_string(res_body, &[&["timezone"]]).unwrap_or_default();
        ip_info.rawIspInfo.readme = first_string(res_body, &[&["readme"]]);

        let mut asn = first_string(
            res_body,
            &[
                &["asn", "asn"],
                &["autonomous_system_number"],
                &["as_number"],
                &["asn"],
            ],
        )
        .unwrap_or_default();
        let mut as_name = first_string(
            res_body,
            &[
                &["asn", "name"],
                &["as_name"],
                &["autonomous_system_organization"],
                &["asn_name"],
                &["organization"],
            ],
        )
        .unwrap_or_default();
        let as_domain = first_string(res_body, &[&["asn", "domain"], &["as_domain"], &["domain"]])
            .unwrap_or_default();
        let mut org = first_string(res_body, &[&["org"]])
            .or_else(|| {
                if !asn.is_empty() || !as_name.is_empty() {
                    Some(format_api_organization(&asn, &as_name, &as_domain))
                } else {
                    None
                }
            })
            .unwrap_or_default();
        if let Some((org_asn, org_name)) = parse_org_asn(&org) {
            if asn.is_empty() {
                asn = org_asn;
            }
            if as_name.is_empty() && !org_name.is_empty() {
                as_name = org_name;
            }
        }
        if org.is_empty() {
            org = format_api_organization(&asn, &as_name, &as_domain);
        }

        ip_info.rawIspInfo.organization = org.clone();
        ip_info.location.city = ip_info.rawIspInfo.city.clone();
        ip_info.location.region = ip_info.rawIspInfo.region.clone();
        ip_info.location.country = ip_info.rawIspInfo.country.clone();
        ip_info.location.country_name =
            first_string(res_body, &[&["countryName"], &["country_name"]]).unwrap_or_default();
        ip_info.location.loc = ip_info.rawIspInfo.location.clone();
        ip_info.location.location = ip_info.rawIspInfo.location.clone();
        ip_info.location.org = org.clone();
        ip_info.location.organization = org.clone();
        ip_info.location.isp = org.clone();
        ip_info.location.postal = ip_info.rawIspInfo.postal.clone();
        ip_info.location.timezone = ip_info.rawIspInfo.timezone.clone();
        ip_info.location.asn = asn;
        ip_info.location.as_name = as_name;
        ip_info.location.as_domain = as_domain;
        ip_info.location.source = "ipinfo".to_string();

        if let Some((latitude, longitude)) = parse_coordinates(&ip_info.rawIspInfo.location) {
            ip_info.location.latitude = Some(latitude);
            ip_info.location.longitude = Some(longitude);
        }

        ip_info.processedString = Self::build_processed_string(
            &ip_info.rawIspInfo.ip,
            &ip_info.rawIspInfo.organization,
            &ip_info.rawIspInfo.country,
            &ip_info.rawIspInfo.city,
            &ip_info.rawIspInfo.region,
            &ip_info.location.country_name,
        );
        ip_info
    }

    pub fn ipinfo_request(ip: &str, token: &str) -> String {
        if token.trim().is_empty() {
            format!(
                "GET /{ip}/json HTTP/1.1\r\n\
Host: ipinfo.io\r\n\
Accept: application/json\r\n\
Connection: close\r\n\r\n"
            )
        } else {
            format!(
                "GET /{ip}/json?token={} HTTP/1.1\r\n\
Host: ipinfo.io\r\n\
Accept: application/json\r\n\
Connection: close\r\n\r\n",
                encode_query_component(token)
            )
        }
    }

    async fn get_ipinfo_information(raw_ip: &str) -> Option<IPInfo> {
        if SERVER_CONFIG
            .get()
            .map(|config| config.disable_ipinfo)
            .unwrap_or(false)
        {
            return None;
        }
        let ip_info_token = SERVER_CONFIG
            .get()
            .map(|config| config.ipinfo_api_key.clone())
            .unwrap_or_default();

        match tokio::time::timeout(Duration::from_secs(5), async {
            if let Ok(mut client) = HttpClient::open("https://ipinfo.io").await {
                let request = Self::ipinfo_request(raw_ip, &ip_info_token);
                if let Ok(Some(res_body)) = client.send_request_json(request.as_bytes()).await {
                    if res_body
                        .get("bogon")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                    {
                        return None;
                    }
                    let ip_info = Self::parse_ipinfo_response(raw_ip, &res_body);
                    if ip_info.has_useful_location_data() {
                        Some(ip_info)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                warn!("Http client error.");
                None
            }
        })
        .await
        {
            Ok(result) => result,
            Err(_) => {
                warn!("IPinfo API lookup timed out");
                None
            }
        }
    }

    #[allow(dead_code)]
    async fn get_isp_info_from_api(ip: &str) -> Option<String> {
        Self::get_ipinfo_information(ip)
            .await
            .map(|ip_info| ip_info.processedString)
    }

    fn has_useful_location_data(&self) -> bool {
        !self.rawIspInfo.organization.is_empty()
            || !self.rawIspInfo.city.is_empty()
            || !self.rawIspInfo.region.is_empty()
            || !self.rawIspInfo.country.is_empty()
            || !self.rawIspInfo.location.is_empty()
            || !self.rawIspInfo.postal.is_empty()
            || !self.rawIspInfo.timezone.is_empty()
            || !self.location.asn.is_empty()
            || !self.location.as_name.is_empty()
            || !self.location.as_domain.is_empty()
    }

    fn merge_offline_location(raw_ip: &str, ip_info: &mut IPInfo) -> bool {
        let mut contributed = false;
        if let Some(city_info) = Self::get_city_info_from_db(raw_ip) {
            let city_country_compatible =
                country_codes_match(&ip_info.location.country, &city_info.country);
            if city_country_compatible {
                if ip_info.rawIspInfo.city.is_empty() {
                    ip_info.rawIspInfo.city = city_info.city.clone();
                    ip_info.location.city = city_info.city.clone();
                    contributed = true;
                }
                if ip_info.rawIspInfo.region.is_empty() {
                    ip_info.rawIspInfo.region = city_info.region.clone();
                    ip_info.location.region = city_info.region.clone();
                    contributed = true;
                }
                if ip_info.rawIspInfo.country.is_empty() {
                    ip_info.rawIspInfo.country = city_info.country.clone();
                    ip_info.location.country = city_info.country.clone();
                    contributed = true;
                }
                if ip_info.rawIspInfo.postal.is_empty() {
                    ip_info.rawIspInfo.postal = city_info.postal.clone();
                    ip_info.location.postal = city_info.postal.clone();
                    contributed = true;
                }
                if ip_info.rawIspInfo.timezone.is_empty() {
                    ip_info.rawIspInfo.timezone = city_info.timezone.clone();
                    ip_info.location.timezone = city_info.timezone.clone();
                    contributed = true;
                }
                if ip_info.location.country_name.is_empty()
                    && country_codes_match(&ip_info.location.country, &city_info.country)
                {
                    ip_info.location.country_name = city_info.country_name.clone();
                    contributed = true;
                }
                if ip_info.location.latitude.is_none() {
                    ip_info.location.latitude = city_info.latitude;
                    if city_info.latitude.is_some() {
                        contributed = true;
                    }
                }
                if ip_info.location.longitude.is_none() {
                    ip_info.location.longitude = city_info.longitude;
                    if city_info.longitude.is_some() {
                        contributed = true;
                    }
                }
                if ip_info.location.loc.is_empty() {
                    if let (Some(latitude), Some(longitude)) =
                        (ip_info.location.latitude, ip_info.location.longitude)
                    {
                        ip_info.location.loc = format!("{latitude:.6},{longitude:.6}");
                        ip_info.location.location = ip_info.location.loc.clone();
                        if ip_info.rawIspInfo.location.is_empty() {
                            ip_info.rawIspInfo.location = ip_info.location.loc.clone();
                        }
                        contributed = true;
                    }
                }
            }
        }

        if let Some(country_info) = Self::get_isp_info_from_db(raw_ip) {
            if ip_info.rawIspInfo.country.is_empty() {
                ip_info.rawIspInfo.country = country_info.country.clone();
                ip_info.location.country = country_info.country.clone();
                contributed = true;
            }
            if ip_info.location.country_name.is_empty()
                && country_codes_match(&ip_info.location.country, &country_info.country)
            {
                ip_info.location.country_name = country_info.country_name.clone();
                contributed = true;
            }
            if asn_codes_match(&ip_info.location.asn, &country_info.asn) {
                if ip_info.location.asn.is_empty() {
                    ip_info.location.asn = country_info.asn.clone();
                    contributed = true;
                }
                if ip_info.location.as_name.is_empty() {
                    ip_info.location.as_name = country_info.as_name.clone();
                    contributed = true;
                }
                if ip_info.location.as_domain.is_empty() {
                    ip_info.location.as_domain = country_info.as_domain.clone();
                    contributed = true;
                }
            }
            if ip_info.location.continent.is_empty() {
                ip_info.location.continent = country_info.continent.clone();
                contributed = true;
            }
            if ip_info.location.continent_name.is_empty() {
                ip_info.location.continent_name = country_info.continent_name.clone();
                contributed = true;
            }
            if ip_info.rawIspInfo.organization.is_empty() {
                let organization = format_offline_organization(&country_info);
                ip_info.rawIspInfo.organization = organization.clone();
                ip_info.location.org = organization.clone();
                ip_info.location.organization = organization.clone();
                ip_info.location.isp = organization;
                contributed = true;
            } else {
                ip_info.location.org = ip_info.rawIspInfo.organization.clone();
                ip_info.location.organization = ip_info.rawIspInfo.organization.clone();
                ip_info.location.isp = ip_info.rawIspInfo.organization.clone();
            }
        }
        contributed
    }

    fn get_offline_information(raw_ip: &str) -> Option<IPInfo> {
        let country_info = Self::get_isp_info_from_db(raw_ip);
        let city_info = Self::get_city_info_from_db(raw_ip);
        if country_info.is_none() && city_info.is_none() {
            return None;
        }

        let mut ip_info = IPInfo::default();
        ip_info.rawIspInfo.ip = raw_ip.to_string();
        ip_info.location.ip = raw_ip.to_string();
        ip_info.location.source = "mmdb".to_string();

        if let Some(city_info) = city_info {
            ip_info.rawIspInfo.city = city_info.city.clone();
            ip_info.rawIspInfo.region = city_info.region.clone();
            ip_info.rawIspInfo.country = city_info.country.clone();
            ip_info.rawIspInfo.postal = city_info.postal.clone();
            ip_info.rawIspInfo.timezone = city_info.timezone.clone();
            ip_info.location.city = city_info.city.clone();
            ip_info.location.region = city_info.region.clone();
            ip_info.location.country = city_info.country.clone();
            ip_info.location.country_name = city_info.country_name.clone();
            ip_info.location.postal = city_info.postal.clone();
            ip_info.location.timezone = city_info.timezone.clone();
            ip_info.location.latitude = city_info.latitude;
            ip_info.location.longitude = city_info.longitude;

            if let (Some(latitude), Some(longitude)) = (city_info.latitude, city_info.longitude) {
                let loc = format!("{latitude:.6},{longitude:.6}");
                ip_info.rawIspInfo.location = loc.clone();
                ip_info.location.loc = loc.clone();
                ip_info.location.location = loc;
            }
        }

        if let Some(country_info) = country_info {
            if ip_info.rawIspInfo.country.is_empty() {
                ip_info.rawIspInfo.country = country_info.country.clone();
                ip_info.location.country = country_info.country.clone();
            }
            if ip_info.location.country_name.is_empty() {
                ip_info.location.country_name = country_info.country_name.clone();
            }

            let organization = format_offline_organization(&country_info);
            ip_info.rawIspInfo.organization = organization.clone();
            ip_info.location.org = organization.clone();
            ip_info.location.organization = organization.clone();
            ip_info.location.isp = organization;
            ip_info.location.asn = country_info.asn.clone();
            ip_info.location.as_name = country_info.as_name.clone();
            ip_info.location.as_domain = country_info.as_domain.clone();
            ip_info.location.continent = country_info.continent.clone();
            ip_info.location.continent_name = country_info.continent_name.clone();
        }

        ip_info.processedString = Self::build_processed_string(
            raw_ip,
            &ip_info.rawIspInfo.organization,
            &ip_info.rawIspInfo.country,
            &ip_info.rawIspInfo.city,
            &ip_info.rawIspInfo.region,
            &ip_info.location.country_name,
        );
        Some(ip_info)
    }

    fn build_processed_string(
        ip: &str,
        organization: &str,
        country: &str,
        city: &str,
        region: &str,
        country_name: &str,
    ) -> String {
        let organization = organization.trim();
        let country_label = if !country_name.trim().is_empty() {
            country_name.trim()
        } else {
            country.trim()
        };

        if !organization.is_empty() {
            if !country_label.is_empty() {
                format!("{ip} - {organization}, {country_label}")
            } else {
                format!("{ip} - {organization}")
            }
        } else if !city.is_empty() {
            let mut place = city.trim().to_string();
            if !region.trim().is_empty() {
                place.push_str(", ");
                place.push_str(region.trim());
            }
            if !country_label.is_empty() {
                place.push_str(", ");
                place.push_str(country_label);
            }
            format!("{ip} - {place}")
        } else if !country_label.is_empty() {
            format!("{ip} - {country_label}")
        } else {
            ip.to_string()
        }
    }

    fn get_isp_info_from_db(ip: &str) -> Option<MMDBResult> {
        let path = SERVER_CONFIG
            .get()
            .map(|config| config.country_asn_mmdb_path.trim().to_string())
            .filter(|path| !path.is_empty())
            .unwrap_or_else(|| "country_asn.mmdb".to_string());
        let cache = COUNTRY_MMDB_CACHE.get_or_init(|| Mutex::new(None));
        let mut cache = cache.lock().ok()?;
        if cache
            .as_ref()
            .map(|(cached_path, _)| cached_path == &path)
            .unwrap_or(false)
        {
            if let Some((_, reader)) = cache.as_mut() {
                return reader.lookup(ip);
            }
        }
        if let Some(reader) = MMDBReader::from(&path) {
            *cache = Some((path.clone(), reader));
            if let Some((_, reader)) = cache.as_mut() {
                return reader.lookup(ip);
            }
        }
        warn!("Unable to open country/ASN MMDB database file: {path}");
        None
    }

    fn get_city_info_from_db(ip: &str) -> Option<MMDBCityResult> {
        let path = SERVER_CONFIG
            .get()
            .map(|config| config.city_mmdb_path.trim().to_string())
            .unwrap_or_default();
        if path.is_empty() {
            return None;
        }
        let cache = CITY_MMDB_CACHE.get_or_init(|| Mutex::new(None));
        let mut cache = cache.lock().ok()?;
        if cache
            .as_ref()
            .map(|(cached_path, _)| cached_path == &path)
            .unwrap_or(false)
        {
            if let Some((_, reader)) = cache.as_mut() {
                return reader.lookup(ip);
            }
        }
        if let Some(reader) = CityMMDBReader::from(&path) {
            *cache = Some((path.clone(), reader));
            if let Some((_, reader)) = cache.as_mut() {
                return reader.lookup(ip);
            }
        }
        warn!("Unable to open optional city MMDB database file: {path}");
        None
    }

    pub fn warm_mmdb_readers() {
        let country_path = SERVER_CONFIG
            .get()
            .map(|config| config.country_asn_mmdb_path.trim().to_string())
            .filter(|path| !path.is_empty())
            .unwrap_or_else(|| "country_asn.mmdb".to_string());
        if let Some(reader) = MMDBReader::from(&country_path) {
            if let Ok(mut cache) = COUNTRY_MMDB_CACHE.get_or_init(|| Mutex::new(None)).lock() {
                *cache = Some((country_path.clone(), reader));
            }
        } else {
            warn!("Unable to open country/ASN MMDB database file: {country_path}");
        }

        let city_path = SERVER_CONFIG
            .get()
            .map(|config| config.city_mmdb_path.trim().to_string())
            .unwrap_or_default();
        if city_path.is_empty() {
            return;
        }
        if let Some(reader) = CityMMDBReader::from(&city_path) {
            if let Ok(mut cache) = CITY_MMDB_CACHE.get_or_init(|| Mutex::new(None)).lock() {
                *cache = Some((city_path, reader));
            }
        } else {
            warn!("Unable to open optional city MMDB database file: {city_path}");
        }
    }

    fn get_local_or_private_info(ip: &str) -> Option<String> {
        if "::1" == ip {
            return Some("localhost IPv6 access".to_string());
        }
        if ip.starts_with("fe80:") {
            return Some("link-local IPv6 access".to_string());
        }
        if Self::is_ula_ipv6(ip) {
            return Some("ULA IPv6 access".to_string());
        }
        if ip.starts_with("127.") {
            return Some("localhost IPv4 access".to_string());
        }
        if Self::is_private_ipv4(ip) {
            return Some("private IPv4 access".to_string());
        }
        if ip.starts_with("169.254.") {
            return Some("link-local IPv4 access".to_string());
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
            _ => false,
        }
    }
}

fn first_string(value: &Value, paths: &[&[&str]]) -> Option<String> {
    for path in paths {
        if let Some(found) = value_at(value, path) {
            match found {
                Value::String(found) if !found.trim().is_empty() => {
                    return Some(found.trim().to_string());
                }
                Value::Number(found) => return Some(found.to_string()),
                Value::Bool(found) => return Some(found.to_string()),
                _ => {}
            }
        }
    }
    None
}

fn value_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for (index, key) in path.iter().enumerate() {
        current = match current {
            Value::Object(object) => object.get(*key)?,
            Value::Array(array) if *key == "0" => array.first()?,
            _ => return None,
        };
        if index + 1 == path.len() {
            return Some(current);
        }
    }
    Some(current)
}

fn parse_coordinates(loc: &str) -> Option<(f64, f64)> {
    let mut parts = loc.split(',').map(str::trim);
    let latitude = parts.next()?.parse::<f64>().ok()?;
    let longitude = parts.next()?.parse::<f64>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    if latitude.is_finite()
        && longitude.is_finite()
        && (-90.0..=90.0).contains(&latitude)
        && (-180.0..=180.0).contains(&longitude)
    {
        Some((latitude, longitude))
    } else {
        None
    }
}

fn format_offline_organization(country_info: &MMDBResult) -> String {
    let as_name = country_info.as_name.trim();
    let asn = country_info.asn.trim();
    let as_domain = country_info.as_domain.trim();
    let asn_label = if asn.is_empty() {
        String::new()
    } else if asn.to_ascii_uppercase().starts_with("AS") {
        asn.to_string()
    } else {
        format!("AS{asn}")
    };

    if !as_name.is_empty() {
        if !asn_label.is_empty() {
            format!("{asn_label} {as_name}")
        } else {
            as_name.to_string()
        }
    } else if !asn_label.is_empty() {
        asn_label
    } else if !as_domain.is_empty() {
        as_domain.to_string()
    } else {
        String::new()
    }
}

fn format_api_organization(asn: &str, as_name: &str, as_domain: &str) -> String {
    let asn = asn.trim();
    let as_name = as_name.trim();
    let as_domain = as_domain.trim();
    let asn_label = if asn.is_empty() {
        String::new()
    } else if asn.to_ascii_uppercase().starts_with("AS") {
        asn.to_string()
    } else {
        format!("AS{asn}")
    };
    if !as_name.is_empty() {
        if !asn_label.is_empty() {
            format!("{asn_label} {as_name}")
        } else {
            as_name.to_string()
        }
    } else if !asn_label.is_empty() {
        asn_label
    } else {
        as_domain.to_string()
    }
}

fn parse_org_asn(org: &str) -> Option<(String, String)> {
    let upper = org.to_ascii_uppercase();
    let position = upper.find("AS")?;
    let suffix = &org[position + 2..];
    let digits: String = suffix
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        return None;
    }

    let after = suffix[digits.len()..]
        .trim()
        .trim_start_matches(['-', '/', '|'])
        .trim();
    let before = org[..position]
        .trim()
        .trim_end_matches(['-', '/', '|'])
        .trim();
    let name = if !after.is_empty() { after } else { before }
        .trim()
        .to_string();

    Some((format!("AS{digits}"), name))
}

fn normalize_asn(asn: &str) -> String {
    let asn = asn.trim();
    if asn.is_empty() {
        return String::new();
    }
    let digits = if asn.len() >= 2 && asn[..2].eq_ignore_ascii_case("AS") {
        &asn[2..]
    } else {
        asn
    };
    digits
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect()
}

fn asn_codes_match(api_asn: &str, offline_asn: &str) -> bool {
    let api = normalize_asn(api_asn);
    let offline = normalize_asn(offline_asn);
    api.is_empty() || (!offline.is_empty() && api == offline)
}

fn normalize_country_code(country: &str) -> String {
    let country = country.trim();
    if country.is_empty() {
        String::new()
    } else {
        country.to_ascii_uppercase()
    }
}

fn country_codes_match(api_country: &str, offline_country: &str) -> bool {
    let api = api_country.trim();
    let offline = offline_country.trim();
    api.is_empty() || offline.is_empty() || api.eq_ignore_ascii_case(offline)
}

fn encode_query_component(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipinfo_request_is_unauthenticated_when_key_is_empty() {
        let request = IPInfo::ipinfo_request("128.94.2.134", "");
        assert!(request.starts_with("GET /128.94.2.134/json "));
        assert!(!request.contains("token="));
    }

    #[test]
    fn ipinfo_request_preserves_api_key_support() {
        let request = IPInfo::ipinfo_request("128.94.2.134", "test key+one");
        assert!(request.contains("token=test%20key%2Bone"));
    }

    #[test]
    fn api_response_populates_raw_and_structured_location() {
        let body = json!({
            "ip": "128.94.2.134",
            "hostname": "reported.example",
            "city": "Los Angeles",
            "region": "California",
            "country": "US",
            "loc": "34.0522,-118.2437",
            "org": "AS10796 Charter Communications Inc",
            "postal": "90001",
            "timezone": "America/Los_Angeles",
            "readme": "https://example.test/readme",
            "country_name": "United States"
        });

        let result = IPInfo::parse_ipinfo_response("128.94.2.134", &body);

        assert_eq!(result.rawIspInfo.ip, "128.94.2.134");
        assert_eq!(result.rawIspInfo.city, "Los Angeles");
        assert_eq!(result.rawIspInfo.region, "California");
        assert_eq!(result.rawIspInfo.country, "US");
        assert_eq!(result.rawIspInfo.location, "34.0522,-118.2437");
        assert_eq!(
            result.rawIspInfo.organization,
            "AS10796 Charter Communications Inc"
        );
        assert_eq!(result.rawIspInfo.postal, "90001");
        assert_eq!(result.rawIspInfo.timezone, "America/Los_Angeles");
        assert_eq!(result.location.city, "Los Angeles");
        assert_eq!(result.location.country_name, "United States");
        assert_eq!(result.location.latitude, Some(34.0522));
        assert_eq!(result.location.longitude, Some(-118.2437));
        assert_eq!(result.location.source, "ipinfo");
        let serialized = serde_json::to_value(&result).unwrap();
        assert!(serialized["location"].is_object());
        assert_eq!(
            serialized["location"]["countryName"].as_str(),
            Some("United States")
        );
    }

    #[test]
    fn api_nested_asn_falls_back_to_organization() {
        let body = json!({
            "ip": "128.94.2.134",
            "country": "US",
            "asn": {
                "asn": 10796,
                "name": "Charter Communications Inc",
                "domain": "spectrum.com"
            }
        });
        let result = IPInfo::parse_ipinfo_response("128.94.2.134", &body);

        assert_eq!(result.location.asn, "10796");
        assert_eq!(result.location.as_name, "Charter Communications Inc");
        assert_eq!(result.location.as_domain, "spectrum.com");
        assert_eq!(
            result.rawIspInfo.organization,
            "AS10796 Charter Communications Inc"
        );
    }

    #[test]
    fn legacy_empty_metadata_deserializes_safely() {
        let result = serde_json::from_str::<IPInfo>(
            r#"{"processedString":"128.94.2.134","rawIspInfo":"","location":""}"#,
        )
        .unwrap();

        assert_eq!(result.processedString, "128.94.2.134");
        assert_eq!(result.rawIspInfo, RawISPInfo::default());
        assert_eq!(result.location, LocationInfo::default());
    }

    #[test]
    fn organization_only_api_result_extracts_asn_and_name() {
        let body = json!({
            "ip": "128.94.2.134",
            "country": "US",
            "org": "AS8075 Microsoft Corporation"
        });
        let result = IPInfo::parse_ipinfo_response("128.94.2.134", &body);

        assert_eq!(result.location.asn, "AS8075");
        assert_eq!(result.location.as_name, "Microsoft Corporation");
    }

    #[test]
    fn invalid_api_coordinates_are_not_copied() {
        let body = json!({
            "ip": "128.94.2.134",
            "country": "US",
            "loc": "999,999",
            "org": "AS8075 Microsoft Corporation"
        });
        let result = IPInfo::parse_ipinfo_response("128.94.2.134", &body);

        assert!(result.rawIspInfo.location.is_empty());
        assert!(result.location.loc.is_empty());
        assert!(result.location.location.is_empty());
        assert!(result.location.latitude.is_none());
        assert!(result.location.longitude.is_none());
    }

    #[test]
    fn location_deserializes_legacy_and_camel_case_country_names() {
        let camel: LocationInfo = serde_json::from_value(json!({
            "countryName": "United States"
        }))
        .unwrap();
        assert_eq!(camel.country_name, "United States");

        let legacy: LocationInfo = serde_json::from_value(json!({
            "country_name": "United States"
        }))
        .unwrap();
        assert_eq!(legacy.country_name, "United States");
    }

    #[test]
    fn coordinates_require_two_bounded_components() {
        assert_eq!(
            parse_coordinates("36.6676,-78.3875"),
            Some((36.6676, -78.3875))
        );
        assert_eq!(parse_coordinates("36,-78,123"), None);
        assert_eq!(parse_coordinates("999,999"), None);
        assert_eq!(parse_coordinates("91,0"), None);
        assert_eq!(parse_coordinates("0,-181"), None);
    }

    #[test]
    fn offline_merge_checks_country_and_asn_matches() {
        assert!(country_codes_match("US", "US"));
        assert!(country_codes_match("", "US"));
        assert!(!country_codes_match("US", "DE"));
        assert!(asn_codes_match("AS8075", "AS8075"));
        assert!(asn_codes_match("", "AS8075"));
        assert!(!asn_codes_match("AS8075", "AS15169"));
    }

    #[test]
    fn mmdb_record_supports_nested_and_flat_shapes() {
        let flat = json!({
            "asn": 10796,
            "as_name": "Charter Communications Inc",
            "as_domain": "spectrum.com",
            "continent": "NA",
            "continent_name": "North America",
            "country": "US",
            "country_name": "United States"
        });
        let parsed = MMDBResult::from_value(&flat).expect("flat MMDB value");
        assert_eq!(parsed.asn, "10796");
        assert_eq!(parsed.as_name, "Charter Communications Inc");

        let nested = json!({
            "autonomous_system_number": 10796,
            "autonomous_system_organization": "Charter Communications Inc",
            "country": {"iso_code": "US", "names": {"en": "United States"}},
            "continent": {"code": "NA", "names": {"en": "North America"}}
        });
        let parsed = MMDBResult::from_value(&nested).expect("nested MMDB value");
        assert_eq!(parsed.asn, "10796");
        assert_eq!(parsed.country, "US");
        assert_eq!(parsed.country_name, "United States");
        assert_eq!(parsed.continent_name, "North America");
    }

    #[test]
    fn city_mmdb_record_supports_city_fields() {
        let value = json!({
            "city": {"names": {"en": "Los Angeles"}},
            "subdivisions": [{"iso_code": "CA", "names": {"en": "California"}}],
            "country": {"iso_code": "US", "names": {"en": "United States"}},
            "postal": {"code": "90001"},
            "location": {
                "latitude": 34.0522,
                "longitude": -118.2437,
                "time_zone": "America/Los_Angeles"
            }
        });
        let parsed = MMDBCityResult::from_value(&value).expect("city MMDB value");
        assert_eq!(parsed.city, "Los Angeles");
        assert_eq!(parsed.region, "California");
        assert_eq!(parsed.country, "US");
        assert_eq!(parsed.country_name, "United States");
        assert_eq!(parsed.postal, "90001");
        assert_eq!(parsed.timezone, "America/Los_Angeles");
        assert_eq!(parsed.latitude, Some(34.0522));
        assert_eq!(parsed.longitude, Some(-118.2437));
    }
}
