#!/usr/bin/env bash
set -eo pipefail

SCRIPTS="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$(dirname "$SCRIPTS")" && pwd)"

id="$1"
if [ ! -f "$ROOT/src/flake/enc/$id" ]; then
  echo "Usage: $0 <id>"
  printf "Available ids:\n%s\n" "$(ls "$ROOT/src/flake/enc")"
  exit 1
fi

nixos-generate \
  --system "aarch64-linux" \
  --format "sd-aarch64" \
  --flake "$ROOT#pidgeon-$id-aarch64-linux"
