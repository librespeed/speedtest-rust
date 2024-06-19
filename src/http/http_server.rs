use std::sync::{Arc};
use log::{info, trace};
use tokio::io::{BufReader, BufWriter};
use tokio::net::{TcpListener};
use tokio::sync::Mutex;
use crate::make_route;
use crate::config::SERVER_CONFIG;
use crate::database::Database;
use crate::http::{check_route_for_index_page, find_remote_ip_addr, get_chunk_count};
use crate::http::request::{handle_socket};
use crate::http::response::{Response};

use crate::http::routes::*;
use crate::ip::ip_info::get_ip_info;
use crate::results::stats::handle_stat_page;

pub struct HttpServer {
    tcp_listener: TcpListener
}

impl HttpServer {

    pub async fn init () -> std::io::Result<Self> {
        let config = SERVER_CONFIG.get().unwrap();
        let addr = format!("{}:{}",config.bind_address,config.listen_port);
        let listener = TcpListener::bind(addr.clone()).await?;
        info!("Server started on {}",addr);
        info!("Server base url : {}",config.base_url);
        Ok(HttpServer { tcp_listener : listener })
    }

    pub async fn listen (&mut self, database : &mut Arc<Mutex<dyn Database + Send>>) {
        loop {
            let tcp_accep = self.tcp_listener.accept().await;
            match tcp_accep {
                Ok((mut socket,_)) => {

                    let database_clone = database.clone();
                    tokio::spawn(async move {

                        let remote_addr = find_remote_ip_addr(&mut socket).await;
                        let (socket_r,socket_w) = socket.split();
                        let mut buff_reader = BufReader::with_capacity(8 * 1024,socket_r);
                        let mut buff_writer = BufWriter::with_capacity(8 * 1024,socket_w);

                        handle_socket(&remote_addr,&mut buff_reader,&mut buff_writer, |request|{

                            let mut database = database_clone.clone();

                            Box::pin(async move {
                                match request.path.trim() {
                                    i if check_route_for_index_page(&request) => {
                                        Response::res_200_index(i)
                                    }
                                    i if i == make_route!("/empty") => {
                                        Response::res_200("")
                                    }
                                    i if i == make_route!("/garbage") => {
                                        let chunks = get_chunk_count(&request.query_params);
                                        Response::res_200_garbage(chunks)
                                    }
                                    i if i == make_route!("/getIP") => {
                                        let ip_info = get_ip_info(&request.remote_addr,request.query_params.get("isp").unwrap_or(&"false".to_string()).parse::<bool>().unwrap_or(false));
                                        Response::res_200_json(&ip_info)
                                    }
                                    i if i == make_route!("/results") => {
                                        show_result_route(&mut database,&request.query_params).await
                                    }
                                    i if i == make_route!("/results/telemetry") => {
                                        telemetry_record_route(&mut database, &request).await
                                    }
                                    i if i == make_route!("/stats") => {
                                        handle_stat_page(&request,&mut database).await
                                    }
                                    _ => {
                                        Response::res_404()
                                    }
                                }
                            })
                        }).await;
                    });
                }
                Err(e) => {
                    trace!("Error tcp connection : {}",e.to_string())
                }
            }
        }
    }

}