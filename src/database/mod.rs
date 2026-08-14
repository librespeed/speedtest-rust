use std::io::Error;
use std::sync::Arc;
use async_trait::async_trait;
use log::info;
use tokio::sync::Mutex;
use uuid::Uuid;
use crate::config::SERVER_CONFIG;
use crate::database::memory::MemoryDB;
#[cfg(feature = "mysql")]
use crate::database::mysql::MySql;
use crate::database::none::NoneDB;
#[cfg(feature = "postgres")]
use crate::database::postgres::Postgres;
#[cfg(feature = "sqlite")]
use crate::database::sqlite::SQLite;
use crate::results::TelemetryData;

#[cfg(feature = "mysql")]
mod mysql;
mod none;
#[cfg(feature = "postgres")]
mod postgres;
#[cfg(feature = "sqlite")]
mod sqlite;
mod memory;

#[async_trait]
pub trait Database {
    async fn insert(&mut self,data : TelemetryData) -> std::io::Result<()>;
    async fn fetch_by_uuid(&mut self,uuid : &str) -> std::io::Result<Option<TelemetryData>>;
    async fn fetch_last_100(&mut self) -> std::io::Result<Vec<TelemetryData>>;
}

pub trait DBRawToStruct<T> {
    fn to_telemetry_struct (&self) -> Result<TelemetryData,T>;
}

pub fn generate_uuid () -> String {
    Uuid::new_v4().to_string()
}

pub async fn init () -> std::io::Result<Arc<Mutex<dyn Database + Send>>> {
    let config = SERVER_CONFIG.get().unwrap();
    match config.database_type.as_str() {
        #[cfg(feature = "mysql")]
        "mysql" => {
            Ok(MySql::init(&config.database_username,&config.database_password,&config.database_hostname,&config.database_name).await?)
        }
        #[cfg(feature = "postgres")]
        "postgres" => {
            Ok(Postgres::init(&config.database_username,&config.database_password,&config.database_hostname,&config.database_name).await?)
        }
        #[cfg(feature = "sqlite")]
        "sqlite" => {
            Ok(SQLite::init(&config.database_file).await?)
        }
        "memory" => {
            Ok(MemoryDB::init())
        }
        "none" => {
            info!("Database disabled");
            Ok(Arc::new(Mutex::new(NoneDB)))
        }
        _ => {
            Err(Error::other("Invalid database type."))
        }
    }
}