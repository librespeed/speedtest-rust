![LibreSpeed Logo](https://github.com/librespeed/speedtest/blob/master/.logo/logo3.png?raw=true)

# speedtest-rust

No Flash, No Java, No WebSocket, No Bullshit.

## Try it
[Take a speed test](https://librespeed-rs.ir)

## Compatibility
Compatible with all librespeed clients :

- Web Client
- [Command line client](https://github.com/librespeed/speedtest-cli)
- [Android client](https://github.com/librespeed/speedtest-android)
- [desktop client](https://github.com/librespeed/speedtest-desktop)

## Attributes
- Memory safety (uses `#![forbid(unsafe_code)]` to ensure everything is implemented in 100% safe Rust.)
- Lightweight & Low overhead
- Low level networking
- Based on tokio-rs (asynchronous)

## Features
- Download
- Upload
- Ping
- Jitter
- IP Address, ISP
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
5. Or setup as service :
    - Copy `setup_systemd.sh` on linux or `setup_sc_win.bat` on windows system in extracted folder.
    - Run the script file to setup as service

[Read full installation methods in wiki](https://github.com/librespeed/speedtest-rust/wiki/Installation)

## Note :
This project can be much better.\
Therefore, your PRs are accepted to improve and solve problems

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
