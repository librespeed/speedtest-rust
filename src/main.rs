#![forbid(unsafe_code)]

use log::error;
use crate::cmd::Cmd;
use crate::http::http_server::HttpServer;

mod http;
mod results;
mod database;
mod ip;
mod config;
mod cmd;

fn main() -> std::io::Result<()> {
    //parse args
    let cmd = Cmd::parse_args();

    if cmd.download_ipdb {
        ip::update_ipdb("https://raw.githubusercontent.com/librespeed/speedtest-rust/master/country_asn.mmdb", "country_asn.mmdb");
        return Ok(())
    }

    //init configs & statics
    if let Err(e) = config::init_configs(cmd) {
        error!("{e}");
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
                                error!("{e}");
                                std::process::exit(1)
                            }
                        }
                    });
                }
                Err(e) => {
                    error!("{e}");
                    std::process::exit(1)
                }
            }
        }
        Err(e) => {
            error!("{e}");
            std::process::exit(1)
        }
    }
    Ok(())
}