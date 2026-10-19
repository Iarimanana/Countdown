#!/usr/bin/env bash
# Builds the Windows installer from Linux: dist/Countdown-Setup-<version>.exe
#
# Needs Rust with the Windows target, plus Zig and cargo-zigbuild:
#   rustup target add x86_64-pc-windows-gnu
#   pip install ziglang && cargo install cargo-zigbuild
set -euo pipefail
cd "$(dirname "$0")"

TARGET=x86_64-pc-windows-gnu
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)

# The installer embeds the app, so the app is built first.
cargo zigbuild --release --target "$TARGET" -p countdown
cargo zigbuild --release --target "$TARGET" -p countdown-setup

mkdir -p dist
cp "target/$TARGET/release/countdown-setup.exe" "dist/Countdown-Setup-$VERSION.exe"
echo "Installer ready: dist/Countdown-Setup-$VERSION.exe"
