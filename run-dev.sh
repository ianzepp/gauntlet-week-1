#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

echo "==> Starting CollabBoard with Docker Compose..."
BUILD_FLAG=""
if [[ "${1:-}" == "--build" ]]; then
  BUILD_FLAG="--build"
  shift
fi

if docker compose version >/dev/null 2>&1; then
  docker compose up ${BUILD_FLAG} "$@"
elif command -v docker-compose >/dev/null 2>&1; then
  docker-compose up ${BUILD_FLAG} "$@"
else
  echo "Docker Compose is not installed (tried 'docker compose' and 'docker-compose')." >&2
  exit 1
fi
