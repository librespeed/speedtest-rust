# syntax=docker/dockerfile:1
# Build
FROM rust:1 AS builder
WORKDIR /usr/local/src

# OS deps
# https://github.com/reproducible-containers/buildkit-cache-dance
ENV DEBIAN_FRONTEND=noninteractive
RUN \
	--mount=type=cache,id=speedtest-rust-debian-apt-cache,target=/var/cache/apt,sharing=locked \
	--mount=type=cache,id=speedtest-rust-debian-apt-lib,target=/var/lib/apt,sharing=locked \
	rm -f /etc/apt/apt.conf.d/docker-clean && \
	echo 'Binary::apt::APT::Keep-Downloaded-Packages "true";' >/etc/apt/apt.conf.d/keep-cache && \
	apt-get update && \
	apt-get install -y mold clang

COPY . .
RUN --mount=type=cache,id=speedtest-rust-debian-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
	--mount=type=cache,id=speedtest-rust-debian-target,target=/usr/local/src/target,sharing=locked \
	RUSTFLAGS="-C linker=clang -C link-arg=-fuse-ld=$(which mold)" \
	cargo install --path . --target-dir target

# Run
FROM debian:bullseye-slim
WORKDIR /usr/local/bin
COPY --from=builder \
	/usr/local/cargo/bin/librespeed-rs \
	librespeed-rs
COPY configs.toml configs.toml
COPY assets assets
EXPOSE 8080
CMD ["librespeed-rs"]
