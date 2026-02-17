#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

echo "==> Building client..."
cd client
bun run build
cd ..

echo "==> Starting server..."
cd server
cargo run
