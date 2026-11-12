#!/usr/bin/env bash
# Builds chess-master for Linux and packs it together with Stockfish into
# target/dist/chess-master-<version>-linux-x86_64.tar.gz
set -euo pipefail

# Which Stockfish release to bundle (see github.com/official-stockfish/Stockfish/releases).
STOCKFISH_TAG="sf_19"
STOCKFISH_ASSET="stockfish-linux-x86-64-universal"
# The download's SHA-256, as GitHub lists it for the release. Any other file is refused.
STOCKFISH_SHA256="9defc0d4e55d49c65a6d042f3e571a39fcea499ade6dbe741b53b8c65e03611f"

cd "$(dirname "$0")/.."
VERSION="$(grep -m1 '^version' Cargo.toml | cut -d '"' -f 2)"
NAME="chess-master-$VERSION-linux-x86_64"
DIST="target/dist"
CACHE="target/stockfish/$STOCKFISH_TAG"

echo "Building chess-master $VERSION..."
cargo build --release --locked

# Download Stockfish once; later runs reuse the copy in target/.
if [[ ! -f "$CACHE/$STOCKFISH_ASSET" ]]; then
  echo "Downloading Stockfish ($STOCKFISH_TAG)..."
  mkdir -p "$CACHE"
  ARCHIVE="$CACHE/$STOCKFISH_ASSET.tar.gz"
  curl --fail --location --silent --show-error --output "$ARCHIVE" \
    "https://github.com/official-stockfish/Stockfish/releases/download/$STOCKFISH_TAG/$STOCKFISH_ASSET.tar.gz"
  # Stop here if the file isn't exactly the expected one (damaged, or replaced by someone).
  echo "$STOCKFISH_SHA256  $ARCHIVE" | sha256sum --check --quiet
  tar --extract --gzip --file "$ARCHIVE" --directory "$CACHE" --strip-components=1 \
    "stockfish/$STOCKFISH_ASSET" "stockfish/Copying.txt"
  rm "$ARCHIVE"
fi

echo "Packing $NAME.tar.gz..."
rm -rf "${DIST:?}/$NAME"
mkdir -p "$DIST/$NAME/licenses"
cp target/release/chess-master "$DIST/$NAME/"
cp "$CACHE/$STOCKFISH_ASSET" "$DIST/$NAME/stockfish"
cp "$CACHE/Copying.txt" "$DIST/$NAME/licenses/Stockfish-COPYING.txt"
cp scripts/linux/install.sh scripts/linux/uninstall.sh "$DIST/$NAME/"
cp assets/icon.png "$DIST/$NAME/chess-master.png"
if [[ -f LICENSE ]]; then
  cp LICENSE "$DIST/$NAME/licenses/chess-master-LICENSE.txt"
fi
cat >"$DIST/$NAME/README.txt" <<README
chess-master $VERSION

Install it (adds it to your app launcher; no sudo needed): ./install.sh
Or just start it from this folder:                         ./chess-master
Remove it again:  ~/.local/share/chess-master/uninstall.sh

The chess engine is Stockfish ($STOCKFISH_TAG), included unchanged as the program
"stockfish" in this folder. Stockfish is free software under the GNU GPL v3
(see licenses/Stockfish-COPYING.txt). Its source code is available at
https://github.com/official-stockfish/Stockfish/tree/$STOCKFISH_TAG
README

tar --create --gzip --file "$DIST/$NAME.tar.gz" --directory "$DIST" "$NAME"
echo "Done: $DIST/$NAME.tar.gz"
