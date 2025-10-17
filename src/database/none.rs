use std::io::Error;
use crate::database::Database;
use crate::results::TelemetryData;

pub struct NoneDB;

impl Database for NoneDB {
    fn insert(&mut self,data : TelemetryData) -> std::io::Result<()> {
        drop(data);
        Err(Error::other("Database disabled"))
    }
    fn fetch_by_uuid(&mut self,_uuid : &str) -> std::io::Result<Option<TelemetryData>> {
        Err(Error::other("Database disabled"))
    }
    fn fetch_last_100(&mut self) -> std::io::Result<Vec<TelemetryData>> {
        Err(Error::other("Database disabled"))
    }
}