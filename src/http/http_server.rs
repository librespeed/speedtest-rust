use std::sync::Arc;
use log::{info, trace};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter, split};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_rustls::TlsAcceptor;
use crate::make_route;
use crate::config::SERVER_CONFIG;
use crate::database::Database;
use crate::http::{check_route_for_index_page, find_remote_ip_addr, get_chunk_count};
use crate::http::request::handle_socket;
use crate::http::response::Response;

use crate::http::routes::*;
use crate::http::tls::setup_tls;
use crate::ip::ip_info::get_ip_info;
use crate::results::stats::handle_stat_page;

pub struct HttpServer {
    pub tcp_listener: TcpListener,
    pub tls_acceptor: Option<TlsAcceptor>
}

impl HttpServer {

    pub async fn init () -> std::io::Result<Self> {
        let config = SERVER_CONFIG.get().unwrap();
        let addr = format!("{}:{}",config.bind_address,config.listen_port);
        let listener = TcpListener::bind(addr.clone()).await?;
        info!("Server started on {}",addr);
        info!("Server base url : {}",config.base_url);
        let mut tls_acceptor = None;
        if config.enable_tls {
            tls_acceptor = Some(setup_tls(&config.tls_cet_file,&config.tls_key_file)?);
        }
        Ok(HttpServer {
            tcp_listener : listener,
            tls_acceptor
        })
    }

    pub async fn listen (&mut self, database : &mut Arc<Mutex<dyn Database + Send>>) {
        loop {

            let tcp_accept = self.tcp_listener.accept().await;
            let mut database = database.clone();
            let tls_acceptor = self.tls_acceptor.clone();

            match tcp_accept {
                Ok((mut socket,_)) => {

                    tokio::spawn(async move {

                        let remote_addr = find_remote_ip_addr(&mut socket).await;

                        if tls_acceptor.is_none() {

                            let (socket_r,socket_w) = socket.split();
                            let mut buff_reader = BufReader::with_capacity(8 * 1024,socket_r);
                            let mut buff_writer = BufWriter::with_capacity(8 * 1024,socket_w);
                            Self::handle_connection(&remote_addr,&mut buff_reader,&mut buff_writer,&mut database).await;


                        } else {

                            let stream = tls_acceptor.unwrap().accept(socket).await;
                            match stream {
                                Ok(stream) => {

                                    let (socket_r, socket_w) = split(stream);
                                    let mut buff_reader = BufReader::with_capacity(8 * 1024, socket_r);
                                    let mut buff_writer = BufWriter::with_capacity(8 * 1024,socket_w);
                                    Self::handle_connection(&remote_addr,&mut buff_reader,&mut buff_writer,&mut database).await;

                                }
                                Err(e) => {
                                    trace!("Error tcp connection : {}",e.to_string())
                                }
                            }

                        }

                    });

                }
                Err(e) => {
                    trace!("Error tcp connection : {}",e.to_string())
                }
            }

        }
    }

    pub async fn handle_connection<R,W>(remote_addr : &str,buf_reader: &mut BufReader<R>,buf_writer : &mut BufWriter<W>,database : &mut Arc<Mutex<dyn Database + Send>>)
        where
            R: AsyncReadExt + Unpin,
            W: AsyncWriteExt + Unpin
    {
        handle_socket(remote_addr, buf_reader, buf_writer, |request|{

            let mut database = database.clone();

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
    }

}