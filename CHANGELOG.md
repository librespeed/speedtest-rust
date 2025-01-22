## Version v1.3.7

### What's changed (fixes) ?

- Fixed remote addr (on Caddy reverse proxy) problem, Thanks to @jonirrings #16
- Added support systemd socket activator, Reported by @drewwells #14

## Version v1.3.6

### What's changed (fixes) ?

- Added support redacting ip addresses
- Added support for result image themes
- Updated CI pipeline to publish Docker images to GHCR, Thanks to @mickkael #9
- Updated CI pipeline to build deb package for easier use in Debian based Linuxes
- Fix show human-readable datetime in stat page, Reported by @shraik #11

## Version v1.3.4

### What's changed (fixes) ?

- Added support for command arguments
- Renamed `speed_test_dir` configs option to `assets_path`, Thanks to @snaakey #7

## Version v1.3.2

### What's changed (fixes) ?

- Added support dual stack socket, Reported by @everything411 #3
- Added support compile with docker compose, Thanks to @ki9us #4
- Minor changes and improvements

## Version v1.3.0

### What's changed (fixes) ?

- Added support for in-memory database
- Included default web-frontend in binary (with automatic server detection)
- Fix some bugs in http server / client
- Fix css loading problem
- Now it is real OOTB ☺️

## Version v1.2.1

### What's changed (fixes) ?

- Added support for ipinfo.io api
- Uses lower versions of **glibc** for run on more gnu **distros**, reported by @everything411 [#1](https://github.com/librespeed/speedtest-rust/issues/1)
- Added 64-bit apple darwin distro
- Added 32-bit linux gnu distro