use std::io::Error;
use std::sync::Arc;
use log::info;
use tokio::sync::Mutex;
use uuid::Uuid;
use crate::config::SERVER_CONFIG;
use crate::database::memory::MemoryDB;
use crate::database::mysql::MySql;
use crate::database::none::NoneDB;
use crate::database::postgres::Postgres;
use crate::database::sqlite::SQLite;
use crate::results::TelemetryData;

mod mysql;
mod none;
mod postgres;
mod sqlite;
mod memory;

pub trait Database {
    fn insert(&mut self,data : TelemetryData) -> std::io::Result<()>;
    fn fetch_by_uuid(&mut self,uuid : &str) -> std::io::Result<Option<TelemetryData>>;
    fn fetch_last_100(&mut self) -> std::io::Result<Vec<TelemetryData>>;
}

pub trait DBRawToStruct<T> {
    fn to_telemetry_struct (&self) -> Result<TelemetryData,T>;
}

pub fn generate_uuid () -> String {
    Uuid::new_v4().to_string()
}

pub fn init () -> std::io::Result<Arc<Mutex<dyn Database + Send>>> {
    let config = SERVER_CONFIG.get().unwrap();
    match config.database_type.as_str() {
        "mysql" => {
            let mysql_setup = mysql::init(&config.database_username,&config.database_password,&config.database_hostname,&config.database_name)?;
            info!("Database {} initialized successfully","Mysql");
            Ok(Arc::new(Mutex::new(MySql{connection : mysql_setup})))
        }
        "postgres" => {
            let postgres_setup = postgres::init(&config.database_username,&config.database_password,&config.database_hostname,&config.database_name)?;
            info!("Database {} initialized successfully","Postgres");
            Ok(Arc::new(Mutex::new(Postgres {connection : postgres_setup})))
        }
        "sqlite" => {
            let sqlite_setup = sqlite::init(&config.database_file)?;
            info!("Database {} initialized successfully","Sqlite");
            Ok(Arc::new(Mutex::new(SQLite {connection : sqlite_setup})))
        }
        "memory" => {
            let memory_setup = memory::init();
            info!("Database {} initialized successfully","in-memory");
            Ok(Arc::new(Mutex::new(MemoryDB {records : memory_setup})))
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