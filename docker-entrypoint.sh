#!/bin/sh
set -e

if command -v bgutil-pot >/dev/null 2>&1; then
  echo "Starting bgutil POT provider"
  bgutil-pot server >/tmp/bgutil-pot.log 2>&1 &
fi

echo "Starting Rust API"
exec /usr/local/bin/app
