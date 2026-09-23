#!/bin/sh
# Bootstrap Rust and build the Wasm bundle for Cloudflare Workers Builds.
#
# The hosted image has no Rust toolchain. `npm run build` prepares it here; deploy.sh restores
# its environment in Cloudflare's separate deploy shell so Wrangler can run its custom build.
set -eu

if ! command -v rustup >/dev/null 2>&1; then
  rustup_installer="${TMPDIR:-/tmp}/xkcdwat-rustup.sh"
  curl --proto '=https' --tlsv1.2 --fail --silent --show-error https://sh.rustup.rs \
    --output "$rustup_installer"
  sh "$rustup_installer" -y --profile minimal --default-toolchain stable
fi

if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck source=/dev/null
  . "$HOME/.cargo/env"
fi

export RUSTUP_TOOLCHAIN=stable
rustup target add wasm32-unknown-unknown

if ! command -v worker-build >/dev/null 2>&1; then
  cargo install worker-build --version '^0.8' --locked
fi

worker-build --release -- --locked
