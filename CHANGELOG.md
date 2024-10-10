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