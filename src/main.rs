#![forbid(unsafe_code)]

use clap::{Arg, ArgAction, Command};
use log::error;
use crate::http::http_server::HttpServer;

mod http;
mod results;
mod database;
mod ip;
mod config;

const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const PKG_NAME: &str = env!("CARGO_PKG_NAME");
const PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const PKG_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

fn main() -> std::io::Result<()> {
    //parse args
    let args = Command::new(PKG_NAME)
        .version(PKG_VERSION)
        .author(PKG_AUTHORS)
        .about(PKG_DESCRIPTION)
        .arg(Arg::new("server_config_path").short('c').long("config"))
        .arg(Arg::new("update-ipdb")
            .long("update-ipdb")
            .help("Download or update IPInfo country asn database")
            .action(ArgAction::SetTrue))
        .get_matches();

    if args.get_flag("update-ipdb") {
        ip::update_ipdb("https://raw.githubusercontent.com/librespeed/speedtest-rust/master/country_asn.mmdb", "country_asn.mmdb");
        return Ok(())
    }

    //get config path
    let config_path = args.get_one::<String>("server_config_path");

    //init configs & statics
    if let Err(e) = config::init_configs(config_path) {
        error!("{}",e.to_string());
        std::process::exit(1)
    }

    //init database
    let database = database::init();
    match database {
        Ok(mut database) => {
            let runtime = config::init_runtime();
            match runtime {
                Ok(runtime) => {
                    runtime.block_on(async  {
                        let http_server = HttpServer::init().await;
                        match http_server {
                            Ok(mut http_server) => {
                                http_server.listen(&mut database).await;
                            }
                            Err(e) => {
                                error!("{}",e.to_string());
                                std::process::exit(1)
                            }
                        }
                    });
                }
                Err(e) => {
                    error!("{}",e.to_string());
                    std::process::exit(1)
                }
            }
        }
        Err(e) => {
            error!("{}",e.to_string());
            std::process::exit(1)
        }
    }
    Ok(())
}