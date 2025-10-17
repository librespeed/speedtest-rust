use std::fs::File;
use std::io::{BufReader, Error, ErrorKind};
use std::sync::Arc;
use log::info;
use rustls_pemfile::{certs, private_key};
use tokio::net::TcpStream;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio_rustls::rustls::{ClientConfig, RootCertStore, ServerConfig};
use tokio_rustls::{TlsAcceptor, TlsConnector};
use tokio_rustls::client::TlsStream;
use tokio_rustls::rustls::pki_types::ServerName;

/** TLS Configuration */
pub fn setup_tls_acceptor(cert_path : &str,key_path : &str) -> std::io::Result<TlsAcceptor> {
    let certs = load_certs(cert_path)?;
    let key = load_key(key_path)?;
    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|err| Error::new(ErrorKind::InvalidInput, err))?;
    let acceptor = TlsAcceptor::from(Arc::new(config));
    info!("Server TLS successfully configured");
    Ok(acceptor)
}

pub async fn setup_tls_connector(domain : String,tcp_stream: TcpStream) -> TlsStream<TcpStream> {
    let mut root_cert_store = RootCertStore::empty();
    root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = ClientConfig::builder().with_root_certificates(root_cert_store).with_no_client_auth();
    let dns_name = ServerName::try_from(domain).unwrap();
    let connector = TlsConnector::from(Arc::new(config));
    connector.connect(dns_name, tcp_stream).await.unwrap()
}

fn open_file_buf(path : &str,err_msg : &str) -> std::io::Result<File> {
    if let Ok(file) = File::open(path) {
        Ok(file)
    } else {
        Err(Error::other(err_msg))
    }
}

fn load_certs(path: &str) -> std::io::Result<Vec<CertificateDer<'static>>> {
    certs(&mut BufReader::new(open_file_buf(path,"Failed to load tls cert file")?)).collect()
}

fn load_key(path: &str) -> std::io::Result<PrivateKeyDer<'static>> {
    private_key(&mut BufReader::new(open_file_buf(path,"Failed to load tls key file")?)).unwrap()
        .ok_or(Error::other("Failed to load tls key file".to_string()))
}