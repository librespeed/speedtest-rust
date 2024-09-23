use clap::{value_parser, Arg, ArgAction, Command};

#[derive(Debug)]
pub struct Cmd {
    pub download_ipdb : bool,
    pub server_config_path : Option<String>,
    pub bind_address : Option<String>,
    pub listen_port : Option<u16>,
    pub base_url : Option<String>,
    pub ipinfo_api_key : Option<String>,
    pub speed_test_dir : Option<String>,
    pub stats_password : Option<String>,
    pub database_type : Option<String>,
    pub database_hostname : Option<String>,
    pub database_name : Option<String>,
    pub database_username : Option<String>,
    pub database_password : Option<String>,
    pub database_file : Option<String>,
    pub enable_tls : Option<bool>,
    pub tls_cert_file : Option<String>,
    pub tls_key_file : Option<String>,
}

const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const PKG_NAME: &str = env!("CARGO_PKG_NAME");
const PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const PKG_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

impl Cmd {

    pub fn parse_args() -> Self {
        let args = Command::new(PKG_NAME)
            .version(PKG_VERSION)
            .author(PKG_AUTHORS)
            .about(PKG_DESCRIPTION)
            .arg(
                Arg::new("server-config-path")
                    .short('c')
                    .long("config")
            )
            .arg(
                Arg::new("update-ipdb")
                    .long("update-ipdb")
                    .help("Download or update IPInfo country asn database")
                    .action(ArgAction::SetTrue)
            )
            .arg(
                Arg::new("bind-address")
                    .short('b')
                    .long("bind-address")
                    .help("Bind IP address")
                    .value_parser(value_parser!(String))
            )
            .arg(
                Arg::new("listen-port")
                    .short('p')
                    .long("listen-port")
                    .help("Listening port")
                    .value_parser(value_parser!(u16))
            )
            .arg(
                Arg::new("base-url")
                    .long("base-url")
                    .help("Specify base api url /{base_url}/routes")
                    .value_parser(value_parser!(String))
            )
            .arg(
                Arg::new("ipinfo-api-key")
                    .long("ipinfo-api-key")
                    .help("Specify the ipinfo API key")
                    .value_parser(value_parser!(String))
            )
            .arg(
                Arg::new("speed-test-dir")
                    .long("speed-test-dir")
                    .help("Specify the directory of speedtest web frontend")
                    .value_parser(value_parser!(String))
            )
            .arg(
                Arg::new("stats-password")
                    .long("stats-password")
                    .help("Specify the password for logging into statistics page")
                    .value_parser(value_parser!(String))
            )
            .arg(
                Arg::new("database-type")
                    .long("database-type")
                    .help("Specify the database type : mysql, postgres, sqlite, memory")
                    .value_parser(value_parser!(String))
            )
            .arg(
                Arg::new("database-hostname")
                    .long("database-hostname")
                    .help("Specify the database connection hostname")
                    .value_parser(value_parser!(String))
            )
            .arg(
                Arg::new("database-name")
                    .long("database-name")
                    .help("Specify the database name")
                    .value_parser(value_parser!(String))
            )
            .arg(
                Arg::new("database-username")
                    .long("database-username")
                    .help("Specify the database authentication username")
                    .value_parser(value_parser!(String))
            )
            .arg(
                Arg::new("database-password")
                    .long("database-password")
                    .help("Specify the database authentication password")
                    .value_parser(value_parser!(String))
            )
            .arg(
                Arg::new("database-file")
                    .long("database-file")
                    .help("Specify the database file path (for sqlite database type)")
                    .value_parser(value_parser!(String))
            )
            .arg(
                Arg::new("enable-tls")
                    .long("enable-tls")
                    .help("Enable and use TLS server")
                    .value_parser(value_parser!(bool))
            )
            .arg(
                Arg::new("tls-cert-file")
                    .long("tls-cert-file")
                    .help("Specify the certificate file path")
                    .value_parser(value_parser!(String))
            )
            .arg(
                Arg::new("tls-key-file")
                    .long("tls-key-file")
                    .help("Specify the key file path")
                    .value_parser(value_parser!(String))
            )
            .get_matches();
        let download_ipdb = args.get_flag("update-ipdb");
        let server_config_path : Option<String> = args.get_one::<String>("server-config-path").map(|s| s.to_owned());
        let bind_address : Option<String> = args.get_one::<String>("bind-address").map(|s| s.to_owned());
        let listen_port : Option<u16> = args.get_one::<u16>("listen-port").map(|s| s.to_owned());
        let base_url : Option<String> = args.get_one::<String>("base-url").map(|s| s.to_owned());
        let ipinfo_api_key : Option<String> = args.get_one::<String>("ipinfo-api-key").map(|s| s.to_owned());
        let speed_test_dir : Option<String> = args.get_one::<String>("speed-test-dir").map(|s| s.to_owned());
        let stats_password : Option<String> = args.get_one::<String>("stats-password").map(|s| s.to_owned());
        let database_type : Option<String> = args.get_one::<String>("database-type").map(|s| s.to_owned());
        let database_hostname : Option<String> = args.get_one::<String>("database-hostname").map(|s| s.to_owned());
        let database_name : Option<String> = args.get_one::<String>("database-name").map(|s| s.to_owned());
        let database_username : Option<String> = args.get_one::<String>("database-username").map(|s| s.to_owned());
        let database_password : Option<String> = args.get_one::<String>("database-password").map(|s| s.to_owned());
        let database_file : Option<String> = args.get_one::<String>("database-file").map(|s| s.to_owned());
        let enable_tls : Option<bool> = args.get_one::<bool>("enable-tls").map(|s| s.to_owned());
        let tls_cert_file : Option<String> = args.get_one::<String>("tls-cert-file").map(|s| s.to_owned());
        let tls_key_file : Option<String> = args.get_one::<String>("tls-key-file").map(|s| s.to_owned());
        Cmd {
            download_ipdb,
            server_config_path,
            bind_address,
            listen_port,
            base_url,
            ipinfo_api_key,
            speed_test_dir,
            stats_password,
            database_type,
            database_hostname,
            database_name,
            database_username,
            database_password,
            database_file,
            enable_tls,
            tls_cert_file,
            tls_key_file,
        }
    }

}