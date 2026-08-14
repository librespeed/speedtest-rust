use std::sync::Arc;
use async_trait::async_trait;
use log::info;
use sqlx::{postgres::{PgPool, PgPoolOptions, PgRow}, Row};
use tokio::sync::Mutex;
use crate::database::{Database, DBRawToStruct};
use crate::results::TelemetryData;

pub struct Postgres {
    pool: PgPool,
}

impl Postgres {
    
    pub async fn init(
        username: &Option<String>,
        password: &Option<String>,
        host_name: &Option<String>,
        db_name: &Option<String>,
    ) -> std::io::Result<Arc<Mutex<Postgres>>> {
        if username.is_none() || password.is_none() || host_name.is_none() || db_name.is_none() {
            return Err(std::io::Error::other("Error postgres initialize parameters."));
        }

        let conn_url = format!(
            "postgresql://{}:{}@{}/{}",
            username.clone().unwrap(),
            password.clone().unwrap(),
            host_name.clone().unwrap(),
            db_name.clone().unwrap()
        );

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&conn_url)
            .await;

        match pool {
            Ok(pool) => {
                let create_table = sqlx::query(
                    "CREATE TABLE IF NOT EXISTS speedtest_users (
                    id serial primary key,
                    ip_address text NOT NULL,
                    isp_info text,
                    extra text,
                    user_agent text NOT NULL,
                    lang text NOT NULL,
                    download text,
                    upload text,
                    ping text,
                    jitter text,
                    log text,
                    uuid text,
                    \"timestamp\" bigint
                )").execute(&pool).await;

                match create_table {
                    Ok(_) => {
                        info!("Database Postgres initialized successfully");
                        Ok(Arc::new(Mutex::new(Postgres{ pool })))
                    },
                    Err(e) => Err(std::io::Error::other(format!("Error setup postgres {:?}", e))),
                }
            }
            Err(e) => Err(std::io::Error::other(format!("Error setup postgres {:?}", e))),
        }
        
    }
    
}

impl DBRawToStruct<sqlx::Error> for PgRow {
    fn to_telemetry_struct(&self) -> Result<TelemetryData, sqlx::Error> {
        Ok(TelemetryData {
            ip_address: self.try_get(1)?,
            isp_info: self.try_get(2)?,
            extra: self.try_get(3)?,
            user_agent: self.try_get(4)?,
            lang: self.try_get(5)?,
            download: self.try_get(6)?,
            upload: self.try_get(7)?,
            ping: self.try_get(8)?,
            jitter: self.try_get(9)?,
            log: self.try_get(10)?,
            uuid: self.try_get(11)?,
            timestamp: self.try_get(12)?,
        })
    }
}

#[async_trait]
impl Database for Postgres {
    async fn insert(&mut self, data: TelemetryData) -> std::io::Result<()> {
        let insert = sqlx::query(
            "INSERT INTO speedtest_users \
             (ip_address,isp_info,extra,user_agent,lang,download,upload,ping,jitter,log,uuid,timestamp) \
             VALUES \
             ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)"
        )
            .bind(&data.ip_address)
            .bind(&data.isp_info)
            .bind(&data.extra)
            .bind(&data.user_agent)
            .bind(&data.lang)
            .bind(&data.download)
            .bind(&data.upload)
            .bind(&data.ping)
            .bind(&data.jitter)
            .bind(&data.log)
            .bind(&data.uuid)
            .bind(data.timestamp)
            .execute(&self.pool)
            .await;

        match insert {
            Ok(_) => Ok(()),
            Err(e) => Err(std::io::Error::other(format!("Error insert postgres {:?}", e))),
        }
    }

    async fn fetch_by_uuid(&mut self, uuid: &str) -> std::io::Result<Option<TelemetryData>> {
        let row = sqlx::query("SELECT * FROM speedtest_users WHERE uuid = $1")
            .bind(uuid)
            .fetch_optional(&self.pool)
            .await;

        match row {
            Ok(Some(row)) => match row.to_telemetry_struct() {
                Ok(item) => Ok(Some(item)),
                Err(_) => Ok(None),
            },
            Ok(None) => Ok(None),
            Err(e) => Err(std::io::Error::other(format!("Error select postgres {:?}", e))),
        }
    }

    async fn fetch_last_100(&mut self) -> std::io::Result<Vec<TelemetryData>> {
        let rows = sqlx::query("SELECT * FROM speedtest_users ORDER BY timestamp DESC LIMIT 100")
            .fetch_all(&self.pool)
            .await;

        match rows {
            Ok(rows) => {
                let result: Vec<TelemetryData> = rows
                    .iter()
                    .filter_map(|row| row.to_telemetry_struct().ok())
                    .collect();
                Ok(result)
            }
            Err(e) => Err(std::io::Error::other(format!("Error select postgres {:?}", e))),
        }
    }
}