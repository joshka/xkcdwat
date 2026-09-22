#!/bin/sh
# Deploy with the Rust environment required by Wrangler's custom build.
#
# Workers Builds uses a fresh shell for deployment, losing the PATH set by build.sh.
# Restore it here; pass Wrangler arguments through to support preview configurations.
set -eu

if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck source=/dev/null
  . "$HOME/.cargo/env"
fi

export RUSTUP_TOOLCHAIN=stable
exec ./node_modules/.bin/wrangler deploy "$@"
