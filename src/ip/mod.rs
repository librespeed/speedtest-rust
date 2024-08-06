use serde::{Deserialize, Serialize};

pub mod ip_info;
pub mod mmdb;
pub mod updater;

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
pub struct IPInfo {
    pub processedString : String,
    pub rawIspInfo : RawISPInfo
}

impl IPInfo {
    pub fn empty () -> Self {
        IPInfo {
            processedString : "".to_string(),
            rawIspInfo : RawISPInfo {
                ip : "".to_string(),
                hostname : "".to_string(),
                city : "".to_string(),
                region : "".to_string(),
                country : "".to_string(),
                location : "".to_string(),
                organization : "".to_string(),
                postal : "".to_string(),
                timezone : "".to_string(),
                readme : None
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct RawISPInfo {
    pub ip : String,
    pub hostname : String,
    pub city : String,
    pub region : String,
    pub country : String,
    #[serde(rename = "loc")]
    pub location : String,
    #[serde(rename = "org")]
    pub organization : String,
    pub postal : String,
    pub timezone : String,
    pub readme : Option<String>
}

/*pub async fn get_isp_info(ip : &str) -> Option<&str> {
    let config = SERVER_CONFIG.get().unwrap();
    let ip_info_token = config.ipinfo_api_key.clone();
    if ip_info_token.is_empty() {
        return None
    }
    //config cert store
    let mut root_cert_store = RootCertStore::empty();
    root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = ClientConfig::builder().with_root_certificates(root_cert_store).with_no_client_auth();
    //open tls connector
    let dns_name = ServerName::try_from("ipinfo.io").unwrap();
    let connector = TlsConnector::from(Arc::new(config));
    //open stream
    let socket = TcpStream::connect("ipinfo.io:443").await.unwrap();
    let mut socket = connector.connect(dns_name, socket).await.unwrap();
    //request content
    let req = format!(
        "GET /{}/json?token={} HTTP/1.1\r\n\
        Host: ipinfo.io\r\n\r\n",
        ip,
        ip_info_token
    );
    //write request
    socket.write_all(req.as_bytes()).await.unwrap();
    //read response not EOF (for close notify tls handshake)
    let mut read_data = Vec::new();
    loop {
        let mut response = vec![0; 128];
        let read = socket.read(&mut response).await.unwrap();
        read_data.extend(response);
        if read < 128 { //EOF
            break;
        }
    }
    let parser = String::from_utf8_lossy(&read_data);
    let response = parser.trim_matches(char::from(0));
    if response.starts_with("HTTP/1.1 200") {
        let mut split_body = response.splitn(2,"\r\n\r\n");
        let resp_body = split_body.nth(1).unwrap();
        let parse_json : Value = serde_json::from_str(resp_body).unwrap();
    }

    None
}*/