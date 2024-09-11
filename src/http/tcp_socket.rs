use core::fmt;
use std::fmt::Formatter;
use std::io;
use std::io::{Error, ErrorKind};
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use socket2::{Domain, Type};
use tokio::net::{TcpListener, TcpStream};
use crate::config::ServerConfig;

pub struct TcpSocket {
    tcp_listener: TcpListener,
    addr : TcpAddr
}

impl TcpSocket {
    pub fn bind(config: &ServerConfig) -> io::Result<TcpSocket> {
        let tcp_addr = TcpAddr::new(config)?;
        let socket = socket2::Socket::new(tcp_addr.domain,Type::STREAM,None)?;
        if !tcp_addr.is_only_v6 {
            socket.set_only_v6(false)?;
        }
        socket.set_reuse_address(true)?;
        socket.bind(&tcp_addr.sock_addr.into())?;
        socket.listen(128)?;
        let tcp_listener = TcpListener::from_std(socket.into())?;
        Ok(TcpSocket {
            tcp_listener,
            addr : tcp_addr
        })
    }

    pub async fn accept (&self) -> io::Result<(TcpStream, SocketAddr)> {
        self.tcp_listener.accept().await
    }

}

impl fmt::Display for TcpSocket {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f,"{}",self.addr.sock_addr)
    }
}

#[derive(Debug)]
pub struct TcpAddr {
    sock_addr: SocketAddr,
    domain: Domain,
    is_only_v6: bool,
}

impl TcpAddr {
    pub fn new(config: &ServerConfig) -> io::Result<Self> {
        let bind_addr = config.bind_address.as_str();
        let parse_addr = Self::parse_addr(bind_addr)?;
        let addr = SocketAddr::new(parse_addr.0, config.listen_port);
        Ok(TcpAddr {
            sock_addr: addr,
            domain: parse_addr.1,
            is_only_v6: bind_addr != "::" && bind_addr != "::0",
        })
    }

    fn parse_addr(ip_str: &str) -> io::Result<(IpAddr, Domain)> {
        match IpAddr::from_str(ip_str) {
            Ok(ip) => match ip {
                IpAddr::V4(_) => Ok((ip, Domain::IPV4)),
                IpAddr::V6(_) => Ok((ip, Domain::IPV6)),
            },
            Err(e) => Err(Error::new(ErrorKind::Other, e)),
        }
    }
}