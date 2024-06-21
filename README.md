![LibreSpeed Logo](https://github.com/librespeed/speedtest/blob/master/.logo/logo3.png?raw=true)

# speedtest-rust

No Flash, No Java, No WebSocket, No Bullshit.

## Compatibility
All modern browsers are supported: IE11, latest Edge, latest Chrome, latest Firefox, latest Safari.
Works with mobile versions too.

## Attributes
- Memory safety
- Lightweight & Low overhead
- Low level networking
- Based on tokio-rs (asynchronous)

## Features
- Download
- Upload
- Ping
- Jitter
- IP Address, ISP, distance from server (optional, not implemented currently)
- Telemetry (optional)
- Results sharing (optional)

## Server requirements
- Any [Rust supported platforms](https://doc.rust-lang.org/beta/rustc/platform-support.html)
- PostgreSQL or MySQL database to store test results (optional)
- A fast! Internet connection

## Installation

### Install using prebuilt binaries

1. Download the appropriate binary file from the [releases](https://github.com/librespeed/speedtest-rust/releases/) page.
2. Unzip the archive.
3. Make changes to the configuration.
4. Run the binary.

### Compile from source

> You need Rust **1.74+** to compile project.\
> So make sure that rust is properly installed on your system

1. Clone the repository:

    ```
    $ git clone https://github.com/librespeed/speedtest-rust.git
    ```

2. Build:

   ```
   # cd to cloned directory
   $ cargo build --release
   ```

3. Prepare files:

   ```
   # Make a new directory
   # Copy target/release/librespeed-rs to created directory
   # Copy configs.toml to created directory
   # If you want the server to load the speedtest web front in the route `/`,
     then you must specify the path of the client folder in the configs.toml file.
   ```
   
4. Change `configs.toml` according to your environment:

    ```toml
    # bind socket adress
    bind_address="0.0.0.0"
    # socket listent port, default is 8080
    listen_port=8080

    # you can specify the number of worker threads. default is 1
    # auto: number of cpu cores, or more than 0 as desired
    # increasing the count increases memory usage
    worker_threads=1
    
    # base api url /{base_url}/routes
    base_url="backend"

    # set directory of speedtest web front to server load on `/`. use empty retrun 404
    speed_test_dir="./assets" # Write without suffix separator

    # password for logging into statistics page, fill this to enable stats page
    stats_password=""

    # database config for : mysql, postgres, sqlite, or disable by write none
    # if none is specified, no telemetry/stats will be recorded, and no result JPG will be generated
    database_type="sqlite"
    database_hostname="localhost"
    database_name="speedtest_db"
    database_username=""
    database_password=""
    # if you use `sqlite` as database, set database_file to database file location
    database_file="speedtest.db"

    # TLS feature comming soon
    # enable_tls=false
    # tls_cet_file=""
    # tls_key_file=""
    ```
   
> #### TODO :
> - [ ] Impl ip geolocation & isp finder
> - [ ] Enable some features like TLS, Docker & more ...

> #### Note : 
> This project can be much better.\
> Therefore, your PRs are accepted to improve and solve problems

## License
Copyright (C) 2016-2024 Federico Dossena\
Copyright (C) 2024 Sudo Dios

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Lesser General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU Lesser General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/lgpl>.