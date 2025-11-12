#!/usr/bin/env bash
set -euo pipefail

# ==============================
# Configuration
# ==============================
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$PROJECT_ROOT"

# Get package metadata (name, version)
if command -v jq >/dev/null 2>&1; then
  PKG_JSON="$(cargo metadata --format-version 1 --no-deps)"
  APP_NAME="$(printf '%s' "$PKG_JSON" | jq -r '.packages[0].name')"
  VERSION="$(printf '%s' "$PKG_JSON" | jq -r '.packages[0].version')"
else
  echo "⚠️  jq not installed, falling back to simple parsing"
  APP_NAME="$(grep -E '^\s*name\s*=' Cargo.toml | head -1 | sed -E 's/.*=\s*"([^"]+)".*/\1/')"
  VERSION="$(grep -E '^\s*version\s*=' Cargo.toml | head -1 | sed -E 's/.*=\s*"([^"]+)".*/\1/')"
fi

DIST_DIR="$PROJECT_ROOT/dist"
mkdir -p "$DIST_DIR"

echo "📦 Building $APP_NAME v$VERSION"
echo "📁 Output directory: $DIST_DIR"

# ==============================
# Helper functions
# ==============================
need_target() { rustup target list --installed | grep -q "^$1$" || rustup target add "$1"; }

ensure_tool() {
  local bin="$1" install_hint="$2"
  if ! command -v "$bin" >/dev/null 2>&1; then
    echo "❌ Missing tool '$bin'. $install_hint"
    exit 2
  fi
}

zipit() {
  local src_bin="$1" out_zip="$2"
  local tmpdir
  tmpdir="$(mktemp -d)"
  local base="$(basename "$src_bin")"
  cp "$src_bin" "$tmpdir/$base"
  [ -f LICENSE ] && cp LICENSE "$tmpdir/LICENSE" || true
  [ -f README.md ] && cp README.md "$tmpdir/README.md" || true
  (cd "$tmpdir" && zip -9r "$out_zip" . >/dev/null)
  rm -rf "$tmpdir"
}

targz() {
  local src_bin="$1" out_tgz="$2"
  local tmpdir
  tmpdir="$(mktemp -d)"
  local base="$(basename "$src_bin")"
  cp "$src_bin" "$tmpdir/$base"
  [ -f LICENSE ] && cp LICENSE "$tmpdir/LICENSE" || true
  [ -f README.md ] && cp README.md "$tmpdir/README.md" || true
  (cd "$tmpdir" && tar -czf "$out_tgz" .)
  rm -rf "$tmpdir"
}

# ==============================
# Tool checks
# ==============================
ensure_tool cargo "Install Rust (rustup)."
ensure_tool rustup "Install Rust (rustup)."
ensure_tool strip "Install Xcode Command Line Tools (xcode-select --install)."
ensure_tool lipo "Install Xcode Command Line Tools."
ensure_tool zig "brew install zig"
ensure_tool cargo-zigbuild "cargo install cargo-zigbuild"
ensure_tool cargo-xwin "cargo install cargo-xwin"

# ==============================
# Targets
# ==============================
MAC_AARCH64="aarch64-apple-darwin"
MAC_X64="x86_64-apple-darwin"
WIN_X64="x86_64-pc-windows-msvc"
LINUX_X64_MUSL="x86_64-unknown-linux-musl"
LINUX_ARM64_MUSL="aarch64-unknown-linux-musl"

for tgt in "$MAC_AARCH64" "$MAC_X64" "$WIN_X64" "$LINUX_X64_MUSL" "$LINUX_ARM64_MUSL"; do
  need_target "$tgt"
done

# ==============================
# macOS builds (arm64 + x86_64 + universal2)
# ==============================
echo "🍏 Building macOS arm64"
cargo build --release --target "$MAC_AARCH64"
BIN_MAC_ARM64="target/$MAC_AARCH64/release/$APP_NAME"
strip "$BIN_MAC_ARM64" || true
zipit "$BIN_MAC_ARM64" "$DIST_DIR/${APP_NAME}-${VERSION}-macos-arm64.zip"

echo "🍏 Building macOS x86_64"
cargo build --release --target "$MAC_X64"
BIN_MAC_X64="target/$MAC_X64/release/$APP_NAME"
strip "$BIN_MAC_X64" || true
zipit "$BIN_MAC_X64" "$DIST_DIR/${APP_NAME}-${VERSION}-macos-x86_64.zip"

echo "🔗 Creating universal2 binary"
BIN_UNIV="target/universal2/$APP_NAME"
mkdir -p "$(dirname "$BIN_UNIV")"
lipo -create -output "$BIN_UNIV" "$BIN_MAC_ARM64" "$BIN_MAC_X64"
strip "$BIN_UNIV" || true
zipit "$BIN_UNIV" "$DIST_DIR/${APP_NAME}-${VERSION}-macos-universal2.zip"

# ==============================
# Windows build (MSVC via cargo-xwin)
# ==============================
echo "🪟 Building Windows x86_64 (MSVC)"
cargo xwin build --release --target "$WIN_X64"
BIN_WIN="target/$WIN_X64/release/${APP_NAME}.exe"
zig objcopy --strip-all "$BIN_WIN" "$BIN_WIN.stripped" && mv "$BIN_WIN.stripped" "$BIN_WIN" || true
zipit "$BIN_WIN" "$DIST_DIR/${APP_NAME}-${VERSION}-windows-x86_64.zip"

# ==============================
# Linux builds (musl, static)
# ==============================
echo "🐧 Building Linux x86_64 (musl, static)"
cargo zigbuild --release --target "$LINUX_X64_MUSL"
BIN_LNX_X64="target/$LINUX_X64_MUSL/release/$APP_NAME"
zig strip "$BIN_LNX_X64" || true
targz "$BIN_LNX_X64" "$DIST_DIR/${APP_NAME}-${VERSION}-linux-x86_64-musl.tar.gz"

echo "🐧 Building Linux aarch64 (musl, static)"
cargo zigbuild --release --target "$LINUX_ARM64_MUSL"
BIN_LNX_ARM64="target/$LINUX_ARM64_MUSL/release/$APP_NAME"
zig strip "$BIN_LNX_ARM64" || true
targz "$BIN_LNX_ARM64" "$DIST_DIR/${APP_NAME}-${VERSION}-linux-aarch64-musl.tar.gz"

# ==============================
# Summary
# ==============================
echo
echo "✅ Build complete. Artifacts:"
ls -1 "$DIST_DIR"
