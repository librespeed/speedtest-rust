use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::path::Path;
use std::sync::OnceLock;

use ab_glyph::FontRef;
use include_dir::{include_dir, Dir};
use log::{info, LevelFilter, trace};
use serde::Deserialize;
use serde_json::Value;
use tokio::runtime::{Builder, Runtime};
use std::io::Write;
use crate::cmd::Cmd;
use crate::config::time::current_formatted_time;

pub mod time;

trait SetIfSome<T> {
    fn set_if_some(&mut self, option: Option<T>);
}

impl<T> SetIfSome<T> for T {
    fn set_if_some(&mut self, option: Option<T>) {
        if let Some(value) = option {
            *self = value;
        }
    }
}

impl<T> SetIfSome<T> for Option<T> {
    fn set_if_some(&mut self, option: Option<T>) {
        if let Some(value) = option {
            *self = Some(value);
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct ServerConfig {
    pub bind_address : String,
    pub listen_port : u16,
    pub worker_threads: Value,
    pub base_url : String,
    pub ipinfo_api_key : String,
    pub stats_password : String,
    pub redact_ip_addresses : bool,
    pub result_image_theme : String,
    pub assets_path : String,
    pub database_type : String,
    pub database_hostname : Option<String>,
    pub database_name : Option<String>,
    pub database_username : Option<String>,
    pub database_password : Option<String>,
    pub database_file : Option<String>,
    pub enable_tls : bool,
    pub tls_cert_file : String,
    pub tls_key_file : String
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            bind_address: "0.0.0.0".to_string(),
            listen_port: 8080,
            worker_threads: Value::from(1),
            base_url: "backend".to_string(),
            ipinfo_api_key: "".to_string(),
            stats_password: "".to_string(),
            redact_ip_addresses: false,
            result_image_theme: "light".to_string(),
            assets_path: "".to_string(),
            database_type: "none".to_string(),
            database_hostname: None,
            database_name: None,
            database_username: None,
            database_password: None,
            database_file: None,
            enable_tls: false,
            tls_cert_file: "".to_string(),
            tls_key_file: "".to_string(),
        }
    }
}

pub fn init_runtime () -> std::io::Result<Runtime> {
    let worker_threads = SERVER_CONFIG.get().unwrap().worker_threads.clone();
    if worker_threads.is_string() && worker_threads == "auto" {
        Builder::new_multi_thread()
            .thread_name("librespeed-rs")
            .enable_io()
            .build()
    } else {
        let mut worker_threads = worker_threads.as_u64().unwrap_or(1) as usize;
        if worker_threads == 0 { worker_threads = 1 }
        Builder::new_multi_thread()
            .thread_name("librespeed-rs")
            .worker_threads(worker_threads)
            .enable_io()
            .build()
    }
}

pub fn init_configs (cmd : Cmd) -> std::io::Result<()> {
    //init logger
    env_logger::builder()
        .format(|buf,rec| {
            let style = buf.default_level_style(rec.level());
            writeln!(buf, "[{} {style}{}{style:#} librespeed_rs] {}",current_formatted_time(),rec.level(), rec.args())
        })
        .filter_level(LevelFilter::Info).init();
    println!("{HEAD_ART}");
    //find server configs
    match cmd.server_config_path.clone() {
        Some(config_path) => {
            let config = open_config_file(&config_path);
            match config {
                Ok(config) => {
                    initialize(config,cmd)?;
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
                    initialize(config,cmd)?;
                    info!("Configs initialized file : configs.toml");
                    Ok(())
                }
                // set default config
                Err(e) => {
                    let config = ServerConfig::default();
                    initialize(config,cmd)?;
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

fn generate_routes(base_url : &str) {
    let mut routes = HashMap::new();
    routes.insert(format!("{base_url}/empty"),"empty");
    routes.insert(format!("{base_url}/garbage"),"garbage");
    routes.insert(format!("{base_url}/getIP"),"getIP");
    routes.insert(format!("{base_url}/results"),"results");
    routes.insert(format!("{base_url}/results/telemetry"),"results/telemetry");
    routes.insert(format!("{base_url}/stats"),"stats");
    ROUTES.get_or_init(|| routes);
}

fn initialize (mut config: ServerConfig,cmd : Cmd) -> std::io::Result<()> {
    //server config
    config.base_url = validate_base_url_path(&config.base_url);
    config.bind_address.set_if_some(cmd.bind_address);
    config.listen_port.set_if_some(cmd.listen_port);
    config.base_url.set_if_some(cmd.base_url);
    config.ipinfo_api_key.set_if_some(cmd.ipinfo_api_key);
    config.assets_path.set_if_some(cmd.assets_path);
    config.stats_password.set_if_some(cmd.stats_password);
    config.redact_ip_addresses.set_if_some(cmd.redact_ip_addresses);
    config.result_image_theme.set_if_some(cmd.result_image_theme);
    config.database_type.set_if_some(cmd.database_type);
    config.database_hostname.set_if_some(cmd.database_hostname);
    config.database_name.set_if_some(cmd.database_name);
    config.database_username.set_if_some(cmd.database_username);
    config.database_password .set_if_some(cmd.database_password);
    config.database_file.set_if_some(cmd.database_file);
    config.enable_tls.set_if_some(cmd.enable_tls);
    config.tls_cert_file.set_if_some(cmd.tls_cert_file);
    config.tls_key_file.set_if_some(cmd.tls_key_file);
    generate_routes(&config.base_url);
    if !config.assets_path.is_empty() {
        if check_assets_path(&config.assets_path) {
            info!("Config assets directory successfully.")
        } else {
            info!("Config assets directory failed !")
        }
    } else {
        info!("Config default assets directory.")
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
    FONT.get_or_init(|| FontRef::try_from_slice(include_bytes!("../../assets/open-sans.ttf")).unwrap());
    Ok(())
}

fn check_assets_path (dir : &str) -> bool {
    let index_file = format!("{}/index.html",dir);
    Path::new(&index_file).exists()
}

/*Static Values*/
const CHUNK_SIZE : usize = 524288; //512 Kilobytes x2
pub static ROUTES: OnceLock<HashMap<String,&str>> = OnceLock::new();
pub static GARBAGE_DATA: OnceLock<Vec<u8>> = OnceLock::new();
pub static SERVER_CONFIG: OnceLock<ServerConfig> = OnceLock::new();
pub static FONT: OnceLock<FontRef> = OnceLock::new();
pub static DEF_ASSETS : Dir = include_dir!("assets");
pub const HEAD_ART : &str = r#"
     _ _ _                                       _
    | (_) |__  _ __ ___  ___ _ __   ___  ___  __| |      _ __ ___
    | | | '_ \| '__/ _ \/ __| '_ \ / _ \/ _ \/ _` |_____| '__/ __|
    | | | |_) | | |  __/\__ \ |_) |  __/  __/ (_| |_____| |  \__ \
    |_|_|_.__/|_|  \___||___/ .__/ \___|\___|\__,_|     |_|  |___/
                            |_|
"#;
