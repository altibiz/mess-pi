#!/usr/bin/env bash
set -eo pipefail

iso="$1"
if [ ! -f "$iso" ]; then
  echo "Usage: $0 <iso> <key>"
  exit 1
fi

key="$2"
if [ ! -f "$key" ]; then
  echo "Usage: $0 <iso> <key>"
  exit 1
fi

temp="$(mktemp -d)"
loop="$(losetup -f)"

losetup "$loop" "$iso"
mount "${loop}p2" "$temp"

mkdir "$temp/root/.sops"
chown root:root "$temp/root/.sops"
chmod 700 "$temp/root/.sops"
cp "$key" "$temp/root/.sops/secrets.age"
chown root:root "$temp/root/.sops/secrets.age"
chmod 600 "$temp/root/.sops/secrets.age"

sudo umount "$temp"
losetup -d "$loop"
