#!/bin/bash

# check system support systemd manager
if ! command -v systemctl &> /dev/null; then
  echo "Error: Systemd is not supported on this system."
  exit 1;
fi

# check script run as sudo
if [ "$EUID" -ne 0 ]
  then echo "Please run script as root"
  exit 1;
fi

# check librespeed-rs exec path
EXEC_PATH=""
if [ ! -f "$1" ] && [ ! -f ./librespeed-rs ]; then
  echo "file not found"
  exit 1;
else
  if [ -n "$1" ]; then
    EXEC_PATH=$(realpath "$1")
  fi
  if [ -f ./librespeed-rs ]; then
    EXEC_PATH=$(realpath ./librespeed-rs)
  fi
fi

# --- Systemd Variables ---
SERVICE_NAME="librespeed-rs"
DESCRIPTION="librespeed rust backend"
WORKING_DIR=$(dirname "$EXEC_PATH")

# --- Create the service file ---
cat << EOF > /etc/systemd/system/${SERVICE_NAME}.service
[Unit]
Description=${DESCRIPTION}
After=network.target

[Service]
Type=simple
ExecStart=${EXEC_PATH}
WorkingDirectory=${WORKING_DIR}
Restart=always
RestartSec=10
StandardOutput=syslog
StandardError=syslog
SyslogIdentifier=%n

[Install]
WantedBy=multi-user.target
EOF

# --- Enable and start the service ---
systemctl daemon-reload
systemctl enable ${SERVICE_NAME}.service
systemctl start ${SERVICE_NAME}.service

echo "librespeed-rs installed and started."