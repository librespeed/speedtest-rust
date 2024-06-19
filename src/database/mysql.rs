use std::io::{Error, ErrorKind};
use mysql::{Conn, Row};
use mysql::prelude::Queryable;
use crate::database::{Database, DBRawToStruct};
use crate::results::TelemetryData;

pub struct MySql {
    pub connection : String
}

pub fn init (username : &Option<String>,password : &Option<String>,host_name : &Option<String>,db_name : &Option<String>) -> std::io::Result<String> {
    if username.is_none() || password.is_none() || host_name.is_none() || db_name.is_none() {
        Err(Error::new(ErrorKind::Other,"Error mysql initialize parameters."))
    } else {
        let conn_url = format!("mysql://{}:{}@{}/{}",username.clone().unwrap(),password.clone().unwrap(),host_name.clone().unwrap(),db_name.clone().unwrap());
        let connection = Conn::new(conn_url.as_str());
        match connection {
            Ok(mut connection) => {
                let create_table = connection.exec_drop("CREATE TABLE IF NOT EXISTS speedtest_users (\
                                    id integer NOT NULL PRIMARY KEY AUTO_INCREMENT,\
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
                                    `timestamp` bigint\
                                )",());
                match create_table {
                    Ok(_) => {
                        drop(connection);
                        Ok(conn_url)
                    }
                    Err(e) => {
                        Err(Error::new(ErrorKind::Other,format!("Error setup mysql {:?}",e)))
                    }
                }
            }
            Err(e) => {
                Err(Error::new(ErrorKind::Other,format!("Error setup mysql {:?}",e)))
            }
        }
    }
}

impl DBRawToStruct<Error> for Row {
    fn to_telemetry_struct(&self) -> Result<TelemetryData,Error> {
        Ok(TelemetryData {
            ip_address : self.get(1).unwrap_or("".to_string()),
            isp_info: self.get(2).unwrap_or("".to_string()),
            extra: self.get(3).unwrap_or("".to_string()),
            user_agent: self.get(4).unwrap_or("".to_string()),
            lang: self.get(5).unwrap_or("".to_string()),
            download: self.get(6).unwrap_or("".to_string()),
            upload: self.get(7).unwrap_or("".to_string()),
            ping: self.get(8).unwrap_or("".to_string()),
            jitter: self.get(9).unwrap_or("".to_string()),
            log: self.get(10).unwrap_or("".to_string()),
            uuid: self.get(11).unwrap_or("".to_string()),
            timestamp: self.get(12).unwrap_or(0),
        })
    }
}

impl Database for MySql {

    fn insert(&mut self,data : TelemetryData) -> std::io::Result<()> {
        let mut connection = Conn::new(self.connection.as_str()).unwrap();
        let insert = connection.exec_drop("INSERT INTO speedtest_users \
                                                (ip_address,isp_info,extra,user_agent,lang,download,upload,ping,jitter,log,uuid,timestamp) \
                                                VALUES \
                                                (?,?,?,?,?,?,?,?,?,?,?,?)",
                                          (&data.ip_address, &data.isp_info, &data.extra, &data.user_agent, &data.lang, &data.download, &data.upload, &data.ping, &data.jitter, &data.log, &data.uuid, &data.timestamp));
        drop(data);
        drop(connection);
        match insert {
            Ok(_) => {
                Ok(())
            }
            Err(e) => {
                Err(Error::new(ErrorKind::Other, format!("Error insert mysql {:?}", e)))
            }
        }
    }

    fn fetch_by_uuid(&mut self,uuid : &str) -> std::io::Result<Option<TelemetryData>> {
        let mut connection = Conn::new(self.connection.as_str()).unwrap();
        let select: Result<Option<Row>, mysql::Error> = connection.exec_first("SELECT * FROM speedtest_users WHERE uuid=?",(uuid,));
        match select {
            Ok(item) => {
                match item {
                    Some(row) => {
                        drop(connection);
                        Ok(Some(row.to_telemetry_struct().unwrap()))
                    }
                    None => {
                        Ok(None)
                    }
                }
            }
            Err(e) => {
                Err(Error::new(ErrorKind::Other, format!("Error select mysql {:?}",e)))
            }
        }
    }

    fn fetch_last_100(&mut self) -> std::io::Result<Vec<TelemetryData>> {
        let mut connection = Conn::new(self.connection.as_str()).unwrap();
        let select: Result<Vec<Row>, mysql::Error> = connection.exec("SELECT * FROM speedtest_users ORDER BY timestamp DESC LIMIT 100",());
        match select {
            Ok(rows) => {
                let result: Vec<TelemetryData> = rows.iter().map(|row| row.to_telemetry_struct().unwrap()).collect();
                drop(connection);
                Ok(result)
            }
            Err(e) => {
                Err(Error::new(ErrorKind::Other, format!("Error select mysql {:?}", e)))
            }
        }
    }

}