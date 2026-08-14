use std::sync::Arc;
use async_trait::async_trait;
use log::info;
use sqlx::{mysql::{MySqlPool, MySqlPoolOptions, MySqlRow}, Row};
use tokio::sync::Mutex;
use crate::database::{Database, DBRawToStruct};
use crate::results::TelemetryData;

pub struct MySql {
    pool: MySqlPool,
}

impl MySql {
    
    pub async fn init(
        username: &Option<String>,
        password: &Option<String>,
        host_name: &Option<String>,
        db_name: &Option<String>,
    ) -> std::io::Result<Arc<Mutex<MySql>>> {
        if username.is_none() || password.is_none() || host_name.is_none() || db_name.is_none() {
            return Err(std::io::Error::other("Error mysql initialize parameters."));
        }

        let conn_url = format!(
            "mysql://{}:{}@{}/{}",
            username.clone().unwrap(),
            password.clone().unwrap(),
            host_name.clone().unwrap(),
            db_name.clone().unwrap()
        );

        let pool = MySqlPoolOptions::new()
            .max_connections(5)
            .connect(&conn_url)
            .await;

        match pool {
            Ok(pool) => {
                let create_table = sqlx::query(
                    "CREATE TABLE IF NOT EXISTS speedtest_users (
                    id integer NOT NULL PRIMARY KEY AUTO_INCREMENT,
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
                    `timestamp` bigint
                )").execute(&pool).await;

                match create_table {
                    Ok(_) => {
                        info!("Database MySql initialized successfully");
                        Ok(Arc::new(Mutex::new(MySql{pool})))
                    },
                    Err(e) => Err(std::io::Error::other(format!("Error setup mysql {:?}", e))),
                }
            }
            Err(e) => Err(std::io::Error::other(format!("Error setup mysql {:?}", e))),
        }
    }
    
}

impl DBRawToStruct<sqlx::Error> for MySqlRow {
    fn to_telemetry_struct(&self) -> Result<TelemetryData, sqlx::Error> {
        Ok(TelemetryData {
            ip_address: self.try_get(1).unwrap_or_default(),
            isp_info: self.try_get(2).unwrap_or_default(),
            extra: self.try_get(3).unwrap_or_default(),
            user_agent: self.try_get(4).unwrap_or_default(),
            lang: self.try_get(5).unwrap_or_default(),
            download: self.try_get(6).unwrap_or_default(),
            upload: self.try_get(7).unwrap_or_default(),
            ping: self.try_get(8).unwrap_or_default(),
            jitter: self.try_get(9).unwrap_or_default(),
            log: self.try_get(10).unwrap_or_default(),
            uuid: self.try_get(11).unwrap_or_default(),
            timestamp: self.try_get(12).unwrap_or_default(),
        })
    }
}

#[async_trait]
impl Database for MySql {
    async fn insert(&mut self, data: TelemetryData) -> std::io::Result<()> {
        let insert = sqlx::query(
            "INSERT INTO speedtest_users \
             (ip_address,isp_info,extra,user_agent,lang,download,upload,ping,jitter,log,uuid,timestamp) \
             VALUES \
             (?,?,?,?,?,?,?,?,?,?,?,?)"
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
            Err(e) => Err(std::io::Error::other(format!("Error insert mysql {:?}", e))),
        }
    }

    async fn fetch_by_uuid(&mut self, uuid: &str) -> std::io::Result<Option<TelemetryData>> {
        let row = sqlx::query("SELECT * FROM speedtest_users WHERE uuid = ?")
            .bind(uuid)
            .fetch_optional(&self.pool)
            .await;

        match row {
            Ok(Some(row)) => match row.to_telemetry_struct() {
                Ok(item) => Ok(Some(item)),
                Err(_) => Ok(None),
            },
            Ok(None) => Ok(None),
            Err(e) => Err(std::io::Error::other(format!("Error select mysql {:?}", e))),
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
            Err(e) => Err(std::io::Error::other(format!("Error select mysql {:?}", e))),
        }
    }
}