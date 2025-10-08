#!/usr/bin/env bash
set -e

# Variables
TRU_BIN="trahl"
PREFIX_BIN="/opt/trahl/bin"
CONFIG_DIR="/opt/trahl/config"
DATA_DIR="/opt/trahl/data"
LOG_DIR="/opt/trahl/log"
SYSTEM_USER="trahl"
SYSTEM_GROUP="trahl"
WORKER_SERVICE_FILE="/etc/systemd/system/trahl-worker.service"
MASTER_SERVICE_FILE="/etc/systemd/system/trahl-master.service"

# Ensure running as root
if [ "$EUID" -ne 0 ]; then
  echo "Please run as root"
  exit 1
fi

if [ ! -f "target/release/$TRU_BIN" ]; then
	echo "Please build project with cargo build --release before installing"
	exit 1
fi

# 1. Create system user and group
if ! id -u $SYSTEM_USER >/dev/null 2>&1; then
  echo "Creating system user $SYSTEM_USER..."
  useradd --system --no-create-home --shell /usr/sbin/nologin $SYSTEM_USER
fi

# 2. Create directories
echo "Creating directories..."
mkdir -p "$CONFIG_DIR" "$DATA_DIR" "$LOG_DIR"

# 3. Set ownership
chown -R $SYSTEM_USER:$SYSTEM_GROUP "$CONFIG_DIR" "$DATA_DIR" "$LOG_DIR"
chmod 755 "$CONFIG_DIR" "$DATA_DIR"
chmod 755 "$LOG_DIR"

# 4. Install trahl binary
echo "Installing $TRU_BIN..."
install -Dm755 "target/release/$TRU_BIN" "$PREFIX_BIN/$TRU_BIN"

# 5. Create systemd service
echo "Creating systemd services..."
cat > "$MASTER_SERVICE_FILE" <<EOF
[Unit]
Description=Trahl Master Daemon
After=network.target

[Service]
Type=simple
User=$SYSTEM_USER
Group=$SYSTEM_GROUP
ExecStart=$PREFIX_BIN/$TRU_BIN --config $CONFIG_DIR/config.toml -m
Restart=on-failure
WorkingDirectory=$DATA_DIR

[Install]
WantedBy=multi-user.target
EOF

cat > "$WORKER_SERVICE_FILE" <<EOF
[Unit]
Description=Trahl Worker Daemon
After=network.target

[Service]
Type=simple
User=$SYSTEM_USER
Group=$SYSTEM_GROUP
ExecStart=$PREFIX_BIN/$TRU_BIN --config $CONFIG_DIR/config.toml -w
Restart=on-failure
WorkingDirectory=$DATA_DIR

[Install]
WantedBy=multi-user.target
EOF

# 6. Reload systemd and enable service
systemctl daemon-reload

echo "Installation complete!"
echo "Use 'systemctl enable --now trahl-master' to enable the master service."
echo "Use 'systemctl enable --now trahl-worker' to enable the worker service."
