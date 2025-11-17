#!/bin/bash
set -euo pipefail

cargo build --release

install -Dm755 target/release/krep /usr/local/bin/krep
install -Dm755 target/release/krep-tray /usr/local/bin/krep-tray
install -Dm644 assets/krep.png /usr/share/icons/hicolor/48x48/apps/krep.png

echo "Install complete"
