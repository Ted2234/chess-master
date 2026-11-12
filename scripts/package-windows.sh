#!/usr/bin/env bash
# Builds chess-master for Windows on this Linux PC and packs it together with Stockfish into
#   target/dist/chess-master-<version>-windows-x86_64.zip        (unzip and run)
#   target/dist/chess-master-<version>-windows-x86_64-setup.exe  (the installer)
set -euo pipefail

# Which Stockfish release to bundle (see github.com/official-stockfish/Stockfish/releases).
STOCKFISH_TAG="sf_19"
STOCKFISH_ASSET="stockfish-windows-x86-64-universal"
# The download's SHA-256, as GitHub lists it for the release. Any other file is refused.
STOCKFISH_SHA256="3c8bf1f9ea66a09350a40df4f632288285ac206d99f33ab5842c408fc30b48a7"
TARGET="x86_64-pc-windows-gnu"

cd "$(dirname "$0")/.."
VERSION="$(grep -m1 '^version' Cargo.toml | cut -d '"' -f 2)"
NAME="chess-master-$VERSION-windows-x86_64"
DIST="target/dist"
CACHE="target/stockfish/$STOCKFISH_TAG"

echo "Building chess-master $VERSION for Windows..."
cargo build --release --locked --target "$TARGET"

# Download Stockfish once; later runs reuse the copy in target/.
if [[ ! -f "$CACHE/$STOCKFISH_ASSET.exe" ]]; then
  echo "Downloading Stockfish ($STOCKFISH_TAG, Windows)..."
  mkdir -p "$CACHE"
  # unzip can't read from a pipe, so the zip is saved first.
  curl --fail --location --silent --show-error --output "$CACHE/windows.zip" \
    "https://github.com/official-stockfish/Stockfish/releases/download/$STOCKFISH_TAG/$STOCKFISH_ASSET.zip"
  # Stop here if the file isn't exactly the expected one (damaged, or replaced by someone).
  echo "$STOCKFISH_SHA256  $CACHE/windows.zip" | sha256sum --check --quiet
  unzip -q -o -j "$CACHE/windows.zip" "stockfish/$STOCKFISH_ASSET.exe" "stockfish/Copying.txt" -d "$CACHE"
  rm "$CACHE/windows.zip"
fi

echo "Packing $NAME..."
rm -rf "${DIST:?}/$NAME"
mkdir -p "$DIST/$NAME/licenses"
cp "target/$TARGET/release/chess-master.exe" "$DIST/$NAME/"
cp "$CACHE/$STOCKFISH_ASSET.exe" "$DIST/$NAME/stockfish.exe"
cp "$CACHE/Copying.txt" "$DIST/$NAME/licenses/Stockfish-COPYING.txt"
cp LICENSE "$DIST/$NAME/licenses/chess-master-LICENSE.txt"
# Windows text files end their lines with \r\n; sed adds the \r.
sed 's/$/\r/' >"$DIST/$NAME/README.txt" <<README
chess-master $VERSION

Start the game: chess-master.exe (keep stockfish.exe next to it).
To install it with a Start menu entry, use chess-master-$VERSION-windows-x86_64-setup.exe.

The chess engine is Stockfish ($STOCKFISH_TAG), included unchanged as stockfish.exe.
Stockfish is free software under the GNU GPL v3 (see licenses\\Stockfish-COPYING.txt).
Its source code is available at
https://github.com/official-stockfish/Stockfish/tree/$STOCKFISH_TAG
README

# 1. The zip: unpack anywhere and run.
rm -f "$DIST/$NAME.zip"
(cd "$DIST" && zip -q -r "$NAME.zip" "$NAME")

# 2. The installer. -NOCD: stay in the project folder, so relative paths work in the script.
makensis -V2 -NOCD \
  -DVERSION="$VERSION" \
  -DSOURCE="$DIST/$NAME" \
  -DOUTFILE="$DIST/$NAME-setup.exe" \
  scripts/windows/installer.nsi
echo "Done: $DIST/$NAME.zip and $DIST/$NAME-setup.exe"
