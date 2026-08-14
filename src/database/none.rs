use std::io::Error;
use async_trait::async_trait;
use crate::database::Database;
use crate::results::TelemetryData;

pub struct NoneDB;

#[async_trait]
impl Database for NoneDB {
    async fn insert(&mut self,data : TelemetryData) -> std::io::Result<()> {
        drop(data);
        Err(Error::other("Database disabled"))
    }
    async fn fetch_by_uuid(&mut self,_uuid : &str) -> std::io::Result<Option<TelemetryData>> {
        Err(Error::other("Database disabled"))
    }
    async fn fetch_last_100(&mut self) -> std::io::Result<Vec<TelemetryData>> {
        Err(Error::other("Database disabled"))
    }
}