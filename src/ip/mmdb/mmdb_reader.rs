use std::net::IpAddr;
use maxminddb::{MaxMindDbError, Reader};
use serde::Deserialize;
use crate::ip::mmdb::mmdb_record::{MMDBRecord, MMDBResult};

pub struct  MMDBReader {
    reader : Reader<Vec<u8>>
}

impl MMDBReader {
    pub fn from(path: &str) -> Option<Self> {
        if let Ok(custom_reader) = maxminddb::Reader::open_readfile(path) {
            Some(MMDBReader {reader : custom_reader})
        } else {
            None
        }
    }
    fn raw_lookup<'a, T: Deserialize<'a>>(&'a self, ip: IpAddr) -> Result<Option<T>, MaxMindDbError> {
        self.reader.lookup(ip)
    }
    pub fn lookup(&mut self,address: &str) -> Option<MMDBResult> {
        if let Ok(Some(result)) = self.raw_lookup::<MMDBRecord>(address.parse().unwrap()) {
            return Some(result.get_result())
        }
        None
    }
}