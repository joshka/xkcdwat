#!/bin/sh
# Build the Rust Worker wherever Wrangler needs a Wasm bundle.
#
# Workers Builds has no Rust toolchain and runs commands in separate shells. Preparing Rust in
# the custom build makes dev, deploy, and preview independent of a previous shell's PATH.
set -eu

if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck source=/dev/null
  . "$HOME/.cargo/env"
fi

if ! command -v rustup >/dev/null 2>&1; then
  rustup_installer="${TMPDIR:-/tmp}/xkcdwat-rustup.sh"
  curl --proto '=https' --tlsv1.2 --fail --silent --show-error https://sh.rustup.rs \
    --output "$rustup_installer"
  sh "$rustup_installer" -y --profile minimal --default-toolchain stable
  # shellcheck source=/dev/null
  . "$HOME/.cargo/env"
fi

export RUSTUP_TOOLCHAIN=stable
rustup target add wasm32-unknown-unknown

if ! command -v worker-build >/dev/null 2>&1; then
  cargo install worker-build --version '^0.8' --locked
fi

worker-build --release -- --locked
