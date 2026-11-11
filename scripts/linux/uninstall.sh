#!/usr/bin/env bash
# Removes everything install.sh installed. Your settings in ~/.config/chess-master are kept.
set -euo pipefail

DATA="${XDG_DATA_HOME:-$HOME/.local/share}"

rm -rf "${DATA:?}/chess-master"
rm -f "$HOME/.local/bin/chess-master" \
  "$DATA/applications/chess-master.desktop" \
  "$DATA/icons/hicolor/256x256/apps/chess-master.png"
update-desktop-database "$DATA/applications" 2>/dev/null || true

echo "chess-master was removed"
echo "your settings are still in ~/.config/chess-master (delete that folder too if you like)."
