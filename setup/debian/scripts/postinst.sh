#!/bin/sh
if [ "$1" = "configure" ] || [ "$1" = "abort-upgrade" ] || [ "$1" = "abort-deconfigure" ] || [ "$1" = "abort-remove" ] ; then
	deb-systemd-helper unmask speedtest_rs.socket >/dev/null || true
	if deb-systemd-helper --quiet was-enabled speedtest_rs.socket; then
		deb-systemd-helper enable speedtest_rs.socket >/dev/null || true
	else
		deb-systemd-helper update-state speedtest_rs.socket >/dev/null || true
	fi
fi