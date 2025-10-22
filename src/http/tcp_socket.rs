use crate::config::ServerConfig;
use core::fmt;
use socket2::{Domain, Type};
use std::fmt::Formatter;
use std::io;
use std::io::{Error, ErrorKind};
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use tokio::net::{TcpListener, TcpStream};
use futures::future::select_all;
use log::info;
use tokio::{select, signal};

pub struct TcpSocket {
    tcp_listeners: Vec<TcpListener>,
    addrs: Vec<TcpAddr>,
    pub(crate) shutdown_tx : tokio::sync::broadcast::Sender<()>,
    from_sys: bool,
}

impl TcpSocket {
    pub fn make_listener(config: &ServerConfig) -> io::Result<TcpSocket> {
        let fd_listeners = Self::find_fd_listeners()?;
        let (tcp_listeners, tcp_addr, from_sys) = if !fd_listeners.0.is_empty() {
            (fd_listeners.0, fd_listeners.1, true)
        } else {
            let tcp_addr = TcpAddr::from_config(config)?;
            (vec![Self::bind(&tcp_addr)?], vec![tcp_addr], false)
        };
        let (shutdown_tx, _) = tokio::sync::broadcast::channel::<()>(1);
        Ok(TcpSocket {
            tcp_listeners,
            addrs: tcp_addr,
            shutdown_tx,
            from_sys,
        })
    }

    fn find_fd_listeners() -> io::Result<(Vec<TcpListener>, Vec<TcpAddr>)> {
        let mut listen_fd = listenfd::ListenFd::from_env();
        let mut fd_listeners = Vec::new();
        let mut addrs = Vec::new();
        if listen_fd.len() > 0 {
            for index in 0..listen_fd.len() {
                if let Ok(Some(listener)) = listen_fd.take_tcp_listener(index) {
                    listener.set_nonblocking(true)?; // PLD Point
                    let tcp_listener = TcpListener::from_std(listener)?;
                    addrs.push(TcpAddr::from_socket(&tcp_listener)?);
                    fd_listeners.push(tcp_listener);
                }
            }
            return Ok((fd_listeners, addrs));
        }
        Ok((Vec::new(), Vec::new()))
    }

    fn bind(tcp_addr: &TcpAddr) -> io::Result<TcpListener> {
        let socket = socket2::Socket::new(tcp_addr.domain, Type::STREAM, None)?;
        if !tcp_addr.is_only_v6 {
            socket.set_only_v6(false)?;
        }
        socket.set_reuse_address(true)?;
        socket.bind(&tcp_addr.sock_addr.into())?;
        socket.listen(128)?;
        socket.set_nonblocking(true)?;
        let tcp_listener = TcpListener::from_std(socket.into())?;
        Ok(tcp_listener)
    }

    pub async fn accept(&self,shutdown_rx: &mut tokio::sync::broadcast::Receiver<()>) -> io::Result<Option<(TcpStream, SocketAddr)>> {
        if self.tcp_listeners.is_empty() {
            return Err(Error::new(
                ErrorKind::NotConnected,
                "No listeners found.",
            ));
        }
    
        let accept_futures = self.tcp_listeners.iter().map(|listener| {
            Box::pin(listener.accept())
        });

        select! {
            res = select_all(accept_futures) => {
                let (result, _index, _remaining) = res;
                result.map(Some)
            }
            _ = shutdown_rx.recv() => Ok(None),
        }
    }

    pub fn spawn_signal_handler(&self) {
        let shutdown_tx = self.shutdown_tx.clone();
        tokio::spawn(async move {
            let ctrl_c = signal::ctrl_c();

            #[cfg(unix)]
            let terminate = async {
                if let Ok(mut term) = signal::unix::signal(signal::unix::SignalKind::terminate()) {
                    term.recv().await;
                }
            };

            #[cfg(not(unix))]
            let terminate = std::future::pending::<()>();

            select! {
                _ = ctrl_c => {},
                _ = terminate => {},
            }

            info!("SIGTERM / Ctrl+C received, notifying tasks ...");
            let _ = shutdown_tx.send(());
        });
    }

}

impl fmt::Display for TcpSocket {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.from_sys {
            true => {
                for (i, addr) in self.addrs.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", addr.sock_addr)?;
                }
                Ok(())
            }
            false => {
                write!(f, "{}", self.addrs[0].sock_addr)
            }
        }
    }
}

#[derive(Debug)]
pub struct TcpAddr {
    sock_addr: SocketAddr,
    domain: Domain,
    is_only_v6: bool,
}

impl TcpAddr {
    pub fn from_config(config: &ServerConfig) -> io::Result<Self> {
        let bind_addr = config.bind_address.as_str();
        let parsed_addr = bind_addr.parse_addr()?;
        let addr = SocketAddr::new(parsed_addr.0, config.listen_port);
        Ok(TcpAddr {
            sock_addr: addr,
            domain: parsed_addr.1,
            is_only_v6: bind_addr != "::" && bind_addr != "::0",
        })
    }

    pub fn from_socket(listener: &TcpListener) -> io::Result<Self> {
        let socket_addr = listener.local_addr()?;
        let parsed_addr = socket_addr.parse_addr()?;
        Ok(TcpAddr {
            sock_addr: socket_addr,
            domain: parsed_addr.1,
            is_only_v6: false,
        })
    }
}

trait IpParser {
    fn parse_addr(&self) -> io::Result<(IpAddr, Domain)>;
}

impl IpParser for &str {
    fn parse_addr(&self) -> io::Result<(IpAddr, Domain)> {
        match IpAddr::from_str(self) {
            Ok(ip) => match ip {
                IpAddr::V4(_) => Ok((ip, Domain::IPV4)),
                IpAddr::V6(_) => Ok((ip, Domain::IPV6)),
            },
            Err(e) => Err(Error::other(e)),
        }
    }
}

impl IpParser for SocketAddr {
    fn parse_addr(&self) -> io::Result<(IpAddr, Domain)> {
        Ok((self.ip(), if self.is_ipv4() { Domain::IPV4 } else { Domain::IPV6 }))
    }
}