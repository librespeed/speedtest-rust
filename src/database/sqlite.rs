use std::io::Error;
use rusqlite::{Connection, Row};
use crate::database::{Database, DBRawToStruct};
use crate::results::TelemetryData;

pub struct SQLite {
    pub connection: Connection,
}

pub fn init (database_file : &Option<String>) -> std::io::Result<Connection> {
    match database_file {
        None => {
            Err(Error::other("Error setup sqlite invalid database file."))
        }
        Some(database_file) => {
            let connection = Connection::open(database_file);
            match connection {
                Ok(connection) => {
                    let create_table = connection.execute(
                        "CREATE TABLE IF NOT EXISTS speedtest_users (\
                                    id INTEGER PRIMARY KEY,\
                                    ip_address TEXT,\
                                    isp_info TEXT,\
                                    extra TEXT,\
                                    user_agent TEXT,\
                                    lang TEXT,\
                                    download TEXT,\
                                    upload TEXT,\
                                    ping TEXT,\
                                    jitter TEXT,\
                                    log TEXT,\
                                    uuid TEXT,\
                                    timestamp INTEGER\
                                )",
                        (),
                    );
                    match create_table {
                        Ok(_) => {
                            Ok(connection)
                        }
                        Err(e) => {
                            Err(Error::other(format!("Error setup sqlite {:?}",e)))
                        }
                    }
                }
                Err(e) => {
                    Err(Error::other(format!("Error setup sqlite {:?}",e)))
                }
            }
        }
    }
}

impl DBRawToStruct<rusqlite::Error> for Row<'_> {
    fn to_telemetry_struct(&self) -> Result<TelemetryData,rusqlite::Error> {
        Ok(TelemetryData {
            ip_address: self.get(1)?,
            isp_info: self.get(2)?,
            extra: self.get(3)?,
            user_agent: self.get(4)?,
            lang: self.get(5)?,
            download: self.get(6)?,
            upload: self.get(7)?,
            ping: self.get(8)?,
            jitter: self.get(9)?,
            log: self.get(10)?,
            uuid: self.get(11)?,
            timestamp: self.get(12)?,
        })
    }
}

impl Database for SQLite {
    fn insert(&mut self, data: TelemetryData) -> std::io::Result<()> {
        let insert = self.connection.execute("INSERT INTO speedtest_users \
                                                (ip_address,isp_info,extra,user_agent,lang,download,upload,ping,jitter,log,uuid,timestamp) \
                                                VALUES \
                                                (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
                                             (&data.ip_address, &data.isp_info, &data.extra, &data.user_agent, &data.lang, &data.download, &data.upload, &data.ping, &data.jitter, &data.log, &data.uuid, &data.timestamp));
        drop(data);
        match insert {
            Ok(_) => {
                Ok(())
            }
            Err(e) => {
                Err(Error::other(format!("Error insert sqlite {:?}", e)))
            }
        }
    }

    fn fetch_by_uuid(&mut self, uuid: &str) -> std::io::Result<Option<TelemetryData>> {
        let select = self.connection.prepare("SELECT * FROM speedtest_users WHERE uuid=?1");
        match select {
            Ok(mut select) => {
                let item = select.query_row([uuid], |row| row.to_telemetry_struct());
                match item {
                    Ok(item) => {
                        Ok(Some(item))
                    }
                    Err(_) => {
                        Ok(None)
                    }
                }
            }
            Err(e) => {
                Err(Error::other(format!("Error select sqlite {:?}", e)))
            }
        }
    }

    fn fetch_last_100(&mut self) -> std::io::Result<Vec<TelemetryData>> {
        let select = self.connection.prepare("SELECT * FROM speedtest_users ORDER BY timestamp DESC LIMIT 100");
        match select {
            Ok(mut select) => {
                let items = select.query_map([], |row| { row.to_telemetry_struct() });
                match items {
                    Ok(items) => {
                        let result: Vec<TelemetryData> = items.map(|row| row.unwrap()).collect();
                        Ok(result)
                    }
                    Err(_) => {
                        Ok(Vec::new())
                    }
                }
            }
            Err(e) => {
                Err(Error::other(format!("Error select sqlite {:?}", e)))
            }
        }
    }
}