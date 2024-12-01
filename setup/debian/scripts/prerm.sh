#!/bin/sh
if [ -d /run/systemd/system ]; then
	deb-systemd-invoke stop speedtest_rs >/dev/null || true
  systemctl stop speedtest_rs.socket
  systemctl disable speedtest_rs.socket
fi