use tokio::runtime::Runtime;
use crate::http::http_client::HttpClient;

pub mod ip_info;
pub mod mmdb;

pub fn update_ipdb(url : &str,file_name : &str) {
    let runtime = Runtime::new().unwrap();
    runtime.block_on(async {
        let mut client = HttpClient::open(url).await;
        client.download_file(file_name).await;
    });
}