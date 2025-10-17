use std::io::Error;
use postgres::{Client, NoTls, Row};
use tokio::task::block_in_place;
use crate::database::{Database, DBRawToStruct};
use crate::results::TelemetryData;

pub struct Postgres {
    pub connection : Client
}

pub fn init (username : &Option<String>,password : &Option<String>,host_name : &Option<String>,db_name : &Option<String>) -> std::io::Result<Client> {
    if username.is_none() || password.is_none() || host_name.is_none() || db_name.is_none() {
        Err(Error::other("Error postgres initialize parameters."))
    } else {
        let conn_url = format!("postgresql://{}:{}@{}/{}",username.clone().unwrap(),password.clone().unwrap(),host_name.clone().unwrap(),db_name.clone().unwrap());
        block_in_place(|| {
            let client = Client::connect(&conn_url, NoTls);
            match client {
                Ok(mut client) => {
                    let create_table = block_in_place(|| {
                        client.execute(
                            "CREATE TABLE IF NOT EXISTS speedtest_users (\
                            id serial primary key,\
                            ip_address text NOT NULL,\
                            isp_info text,\
                            extra text,\
                            user_agent text NOT NULL,\
                            lang text NOT NULL,\
                            download text,\
                            upload text,\
                            ping text,\
                            jitter text,\
                            log text,\
                            uuid text,\
                            \"timestamp\" bigint\
                            )",&[])
                    });
                    match create_table {
                        Ok(_) => {
                            Ok(client)
                        }
                        Err(e) => {
                            Err(Error::other(format!("Error setup postgres {:?}",e)))
                        }
                    }
                }
                Err(e) => {
                    Err(Error::other(format!("Error setup postgres {:?}",e)))
                }
            }
        })
    }
}

impl DBRawToStruct<Error> for Row {
    fn to_telemetry_struct(&self) -> Result<TelemetryData, Error> {
        Ok(TelemetryData {
            ip_address: self.get(1),
            isp_info: self.get(2),
            extra: self.get(3),
            user_agent: self.get(4),
            lang: self.get(5),
            download: self.get(6),
            upload: self.get(7),
            ping: self.get(8),
            jitter: self.get(9),
            log: self.get(10),
            uuid: self.get(11),
            timestamp: self.get(12),
        })
    }
}

impl Database for Postgres {
    fn insert(&mut self,data : TelemetryData) -> std::io::Result<()> {
        let insert = block_in_place(|| {
            self.connection.execute("INSERT INTO speedtest_users \
                                                (ip_address,isp_info,extra,user_agent,lang,download,upload,ping,jitter,log,uuid,timestamp) \
                                                VALUES \
                                                ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)",
                                    &[&data.ip_address, &data.isp_info, &data.extra, &data.user_agent, &data.lang, &data.download, &data.upload, &data.ping, &data.jitter, &data.log, &data.uuid, &data.timestamp])
        });
        drop(data);
        match insert {
            Ok(_) => {
                Ok(())
            }
            Err(e) => {
                Err(Error::other(format!("Error insert postgres {:?}", e)))
            }
        }
    }
    fn fetch_by_uuid(&mut self,uuid : &str) -> std::io::Result<Option<TelemetryData>> {
        let row = block_in_place(|| {
            self.connection.query_one("SELECT * FROM speedtest_users WHERE uuid=$1",&[&uuid.to_string()])
        });
        match row {
            Ok(row) => {
                Ok(Some(row.to_telemetry_struct().unwrap()))
            }
            Err(e) => {
                Err(Error::other(format!("Error select postgres {:?}", e)))
            }
        }
    }
    fn fetch_last_100(&mut self) -> std::io::Result<Vec<TelemetryData>> {
        let rows = block_in_place(|| {
            self.connection.query("SELECT * FROM speedtest_users ORDER BY timestamp DESC LIMIT 100",&[])
        });
        match rows {
            Ok(rows) => {
                let result: Vec<TelemetryData> = rows.iter().map(|row| { row.to_telemetry_struct().unwrap() }).collect();
                Ok(result)
            }
            Err(e) => {
                Err(Error::other(format!("Error select postgres {:?}", e)))
            }
        }
    }
}