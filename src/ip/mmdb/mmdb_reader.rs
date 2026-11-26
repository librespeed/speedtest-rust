use std::net::IpAddr;

use log::warn;
use maxminddb::Reader;
use serde_json::Value;

use crate::ip::mmdb::mmdb_record::{MMDBCityResult, MMDBResult};

pub struct MMDBReader {
    reader: Reader<Vec<u8>>,
}

impl MMDBReader {
    pub fn from(path: &str) -> Option<Self> {
        if let Ok(custom_reader) = maxminddb::Reader::open_readfile(path) {
            Some(MMDBReader {
                reader: custom_reader,
            })
        } else {
            None
        }
    }

    pub fn lookup(&mut self, address: &str) -> Option<MMDBResult> {
        let ip = match address.parse::<IpAddr>() {
            Ok(ip) => ip,
            Err(_) => {
                warn!("Invalid IP address for Geo IP lookup: {}", address);
                return None;
            }
        };
        match self.reader.lookup(ip) {
            Err(e) => {
                warn!("Geo IP database error: {}", e);
                None
            }
            Ok(lookup) => {
                if let Ok(Some(value)) = lookup.decode::<Value>() {
                    if let Some(result) = MMDBResult::from_value(&value) {
                        return Some(result);
                    }
                }
                warn!("Failed to deserialise Geo IP data");
                None
            }
        }
    }
}

pub struct CityMMDBReader {
    reader: Reader<Vec<u8>>,
}

impl CityMMDBReader {
    pub fn from(path: &str) -> Option<Self> {
        if let Ok(custom_reader) = maxminddb::Reader::open_readfile(path) {
            Some(CityMMDBReader {
                reader: custom_reader,
            })
        } else {
            None
        }
    }

    pub fn lookup(&mut self, address: &str) -> Option<MMDBCityResult> {
        let ip = match address.parse::<IpAddr>() {
            Ok(ip) => ip,
            Err(_) => {
                warn!("Invalid IP address for city Geo IP lookup: {}", address);
                return None;
            }
        };
        match self.reader.lookup(ip) {
            Err(e) => {
                warn!("City Geo IP database error: {}", e);
                None
            }
            Ok(lookup) => {
                if let Ok(Some(value)) = lookup.decode::<Value>() {
                    MMDBCityResult::from_value(&value)
                } else {
                    warn!("Failed to deserialise city Geo IP data");
                    None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MMDBReader;

    #[test]
    fn bundled_mmdb_handles_reported_ip_without_panicking() {
        let Some(mut reader) = MMDBReader::from("country_asn.mmdb") else {
            return;
        };
        let result = reader
            .lookup("128.94.2.134")
            .expect("bundled MMDB should resolve 128.94.2.134");
        assert_eq!(result.country, "US");
        assert_eq!(result.asn, "AS8075");
        assert!(!result.as_name.is_empty());
        let _ = reader.lookup("not-an-ip");
    }
}
