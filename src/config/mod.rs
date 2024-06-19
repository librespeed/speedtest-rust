use std::env;
use std::io::{Error, ErrorKind};
use std::path::Path;
use std::sync::OnceLock;

use ab_glyph::FontRef;
use log::{info, trace};
use serde::Deserialize;

pub mod time;

#[derive(Deserialize, Debug)]
pub struct ServerConfig {
    pub bind_address : String,
    pub listen_port : i32,
    pub base_url : String,
    pub stats_password : String,
    pub speed_test_dir : String,
    pub database_type : String,
    pub database_hostname : Option<String>,
    pub database_name : Option<String>,
    pub database_username : Option<String>,
    pub database_password : Option<String>,
    pub database_file : Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            bind_address: "0.0.0.0".to_string(),
            listen_port: 8080,
            base_url: "backend".to_string(),
            stats_password: "".to_string(),
            speed_test_dir: "".to_string(),
            database_type: "none".to_string(),
            database_hostname: None,
            database_name: None,
            database_username: None,
            database_password: None,
            database_file: None,
        }
    }
}

pub fn init_configs (config_path : Option<&String>) -> std::io::Result<()> {
    //init logger
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();
    //find server configs
    match config_path {
        Some(config_path) => {
            let config = open_config_file(config_path);
            match config {
                Ok(config) => {
                    initialize(config)?;
                    info!("Configs initialized file : {}",config_path);
                    Ok(())
                }
                Err(e) => {
                    Err(Error::new(ErrorKind::Other,e))
                }
            }
        }
        None => {
            let config = open_config_file("configs.toml");
            match config {
                // open config from current dir
                Ok(config) => {
                    initialize(config)?;
                    info!("Configs initialized file : configs.toml");
                    Ok(())
                }
                // set default config
                Err(e) => {
                    let config = ServerConfig::default();
                    initialize(config)?;
                    info!("Configs initialized with defaults");
                    trace!("Load config default path error : {}",e);
                    Ok(())
                }
            }
        }
    }
}

fn open_config_file(path : &str) -> std::io::Result<ServerConfig> {
    let config_file_content = std::fs::read_to_string(path); // open file
    match config_file_content {
        Ok(config_file_content) => { // opened file
            let config = toml::from_str(&config_file_content); // parse config file
            match config {
                Ok(config) => {
                    Ok(config)
                }
                Err(e) => {
                    Err(Error::new(ErrorKind::Other,e))
                }
            }
        }
        Err(e) => {
            Err(Error::new(ErrorKind::Other,e))
        }
    }
}

fn validate_base_url_path(base_url : &str) -> String {
    let base_url = base_url.replace('/',"");
    if base_url.is_empty() {
        "".to_string()
    } else {
        format!("/{}",base_url)
    }
}

fn initialize (mut config: ServerConfig) -> std::io::Result<()> {
    //server config
    config.base_url = validate_base_url_path(&config.base_url);
    if !config.speed_test_dir.is_empty() {
        if check_speed_test_dir(&config.speed_test_dir) {
            info!("Config speed test directory successfully.")
        } else {
            info!("Config speed test directory failed !")
        }
    }
    SERVER_CONFIG.get_or_init(|| config);
    //garbage data
    let chunk_len = format!("{:X}\r\n", CHUNK_SIZE);
    let mut garbage = Vec::new();
    garbage.extend(chunk_len.as_bytes());
    garbage.extend(vec![0; CHUNK_SIZE]);
    garbage.extend(b"\r\n");
    GARBAGE_DATA.get_or_init(|| garbage);
    //font for result image
    FONT.get_or_init(|| FontRef::try_from_slice(include_bytes!("../../res/open-sans.ttf")).unwrap());
    Ok(())
}

fn check_speed_test_dir (dir : &str) -> bool {
    let index_file = format!("{}/index.html",dir);
    Path::new(&index_file).exists()
}

/*Static Values*/
const CHUNK_SIZE : usize = 524288; //512 Kilobytes x2
pub static GARBAGE_DATA: OnceLock<Vec<u8>> = OnceLock::new();
pub static SERVER_CONFIG: OnceLock<ServerConfig> = OnceLock::new();
pub static FONT: OnceLock<FontRef> = OnceLock::new();