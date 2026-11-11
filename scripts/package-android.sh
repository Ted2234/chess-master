#!/usr/bin/env bash
# Builds chess-master for Android phones (arm64) and packs it together with Stockfish into
# target/dist/chess-master-<version>-android-arm64.apk
set -euo pipefail

# Which Stockfish release to bundle (see github.com/official-stockfish/Stockfish/releases).
STOCKFISH_TAG="sf_19"
STOCKFISH_ASSET="stockfish-android-arm64-universal"
# The Android tools this script uses (installed with sdkmanager, see Lab 20).
BUILD_TOOLS_VERSION="36.1.0"
NDK_VERSION="29.0.14206865"
PLATFORM="android-36"
# Android 10 is the oldest version Stockfish's Android build runs on.
MIN_SDK=29
TARGET_SDK=35
# The key that signs the app. Keep it (and a backup) outside the repository.
KEYSTORE="${CHESS_MASTER_KEYSTORE:-$HOME/.android/chess-master-release.jks}"

cd "$(dirname "$0")/.."
SDK="${ANDROID_HOME:?Set ANDROID_HOME to your Android SDK folder (Lab 20, Step 1)}"
BUILD_TOOLS="$SDK/build-tools/$BUILD_TOOLS_VERSION"
NDK_BIN="$SDK/ndk/$NDK_VERSION/toolchains/llvm/prebuilt/linux-x86_64/bin"
VERSION="$(grep -m1 '^version' Cargo.toml | cut -d '"' -f 2)"
# Android compares versions by one whole number: 0.2.0 -> 200, 1.4.12 -> 10412.
IFS=. read -r MAJOR MINOR PATCH <<<"$VERSION"
VERSION_CODE=$((MAJOR * 10000 + MINOR * 100 + PATCH))
NAME="chess-master-$VERSION-android-arm64"
DIST="target/dist"
WORK="target/android"
CACHE="target/stockfish/$STOCKFISH_TAG"

if [[ ! -f "$KEYSTORE" ]]; then
  echo "No signing key at $KEYSTORE. Create one first (Lab 20, Step 8)." >&2
  exit 1
fi

echo "Building chess-master $VERSION for Android..."
# Rust compiles the code; the NDK's clang links it for Android.
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$NDK_BIN/aarch64-linux-android$MIN_SDK-clang"
cargo rustc --release --locked --lib --target aarch64-linux-android --crate-type cdylib

# Download Stockfish once; later runs reuse the copy in target/.
if [[ ! -f "$CACHE/$STOCKFISH_ASSET" ]]; then
  echo "Downloading Stockfish ($STOCKFISH_TAG, Android)..."
  mkdir -p "$CACHE"
  curl --fail --location --silent --show-error \
      "https://github.com/official-stockfish/Stockfish/releases/download/$STOCKFISH_TAG/$STOCKFISH_ASSET.tar.gz" |
      tar --extract --gzip --directory "$CACHE" --strip-components=1 \
      "stockfish/$STOCKFISH_ASSET" "stockfish/Copying.txt"
fi

echo "Packing $NAME.apk..."
rm -rf "$WORK"
mkdir -p "$WORK/res/mipmap-xxxhdpi" "$WORK/apk/lib/arm64-v8a" "$WORK/apk/assets/licenses" "$DIST"

# 1. Resources (theme, icon) and the manifest -> an APK without code.
cp -r android/res/. "$WORK/res/"
cp assets/icon.png "$WORK/res/mipmap-xxxhdpi/ic_launcher.png"
"$BUILD_TOOLS/aapt2" compile --dir "$WORK/res" -o "$WORK/res.zip"
"$BUILD_TOOLS/aapt2" link -o "$WORK/unsigned.apk" \
  --manifest android/AndroidManifest.xml \
  -I "$SDK/platforms/$PLATFORM/android.jar" \
  --min-sdk-version "$MIN_SDK" --target-sdk-version "$TARGET_SDK" \
  --version-code "$VERSION_CODE" --version-name "$VERSION" \
  "$WORK/res.zip"

# 2. Add our library, Stockfish (named like a library) and the licenses.
cp target/aarch64-linux-android/release/libchess_master.so "$WORK/apk/lib/arm64-v8a/"
cp "$CACHE/$STOCKFISH_ASSET" "$WORK/apk/lib/arm64-v8a/libstockfish.so"
cp "$CACHE/Copying.txt" "$WORK/apk/assets/licenses/Stockfish-COPYING.txt"
cp LICENSE "$WORK/apk/assets/licenses/chess-master-LICENSE.txt"
(cd "$WORK/apk" && zip -q -r ../unsigned.apk lib assets)

# 3. Align the files inside the APK, then sign it.
"$BUILD_TOOLS/zipalign" -f -P 16 4 "$WORK/unsigned.apk" "$WORK/aligned.apk"
SIGN_ARGS=(--ks "$KEYSTORE")
if [[ -n "${CHESS_MASTER_KEYSTORE_PASSWORD:-}" ]]; then
  SIGN_ARGS+=(--ks-pass env:CHESS_MASTER_KEYSTORE_PASSWORD)
fi
"$BUILD_TOOLS/apksigner" sign "${SIGN_ARGS[@]}" --v4-signing-enabled false --out "$DIST/$NAME.apk" "$WORK/aligned.apk"
echo "Done: $DIST/$NAME.apk"
