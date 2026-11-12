#!/usr/bin/env bash
# Installs chess-master for the current user. No sudo needed; run it again to update.
# the app and Stockfish -> ~/.local/share/chess-master/
# a terminal command    -> ~/.local/bin/chess-master
# the launcher entry    -> ~/.local/share/applications/chess-master.desktop
# the icon              -> ~/.local/share/icons/hicolor/256x256/apps/chess-master.png
set -euo pipefail

SOURCE="$(cd "$(dirname "$0")" && pwd)"
DATA="${XDG_DATA_HOME:-$HOME/.local/share}"
APP_DIR="$DATA/chess-master"
BIN_DIR="$HOME/.local/bin"

echo "Install chess-master to $APP_DIR"
mkdir -p "$APP_DIR/licenses" "$BIN_DIR" "$DATA/applications"
install -m 755 "$SOURCE/chess-master" "$SOURCE/stockfish" "$SOURCE/uninstall.sh" "$APP_DIR/"
install -m 644 "$SOURCE"/licenses/* "$APP_DIR/licenses/"
install -D -m 644 "$SOURCE/chess-master.png" "$DATA/icons/hicolor/256x256/apps/chess-master.png"
ln -sf "$APP_DIR/chess-master" "$BIN_DIR/chess-master"

# The launcher entry needs the full path of the program, so it's written here, not shipped.
# The quotes keep a path with a space in it (e.g. in $HOME) in one piece.
cat >"$DATA/applications/chess-master.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=chess-master
Comment=Play chess against a friend or Stockfish
Exec="$APP_DIR/chess-master"
Icon=chess-master
Terminal=false
Categories=Game;BoardGame;
StartupWMClass=chess-master
DESKTOP

# Tell the desktop about the new entry icon. Not every system has these tools; that's fine.
update-desktop-database "$DATA/applications" 2>/dev/null || true
gtk-update-icon-cache --quiet "$DATA/icons/hicolor" 2>/dev/null || true
echo "Done: Start chess-master from your app launcher, or type: chess-master"
if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
  echo "Note: $BIN_DIR is not on your PATH, so the terminal command won't work (the launcher will)."
fi
