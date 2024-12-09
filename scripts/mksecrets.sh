#!/usr/bin/env bash
set -eo pipefail

cloud_domain="$1"
if [[ "$cloud_domain" == "" ]]; then
  printf "Cloud domain required! Exiting..."
  exit 1
fi

network_ip_range_start="$2"
if [[ "$network_ip_range_start" == "" ]]; then
  printf "Network ip range start required! Exiting..."
  exit 1
fi

network_ip_range_end="$3"
if [[ "$network_ip_range_end" == "" ]]; then
  printf "Network ip range end required! Exiting..."
  exit 1
fi

vpn_ip="$4"
if [[ "$vpn_ip" == "" ]]; then
  printf "Vpn ip required! Exiting..."
  exit 1
fi

ensure() {
  local path

  path="$1"

  if [[ -d "$path" ]]; then
    rm -rf "$path"
  fi

  mkdir -p "$path"
}

SCRIPTS="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$(dirname "$SCRIPTS")" && pwd)"
SECRETS="$ROOT/secrets"
mkdir -p "$SECRETS"
TMP_SECRETS="$SECRETS/tmp"
ensure "$TMP_SECRETS"

ID="$(openssl rand -hex 16)"
ID_SECRETS="$SECRETS/$ID"
if [[ -d "$ID_SECRETS" ]]; then
  printf "Device secrets already exist! Please try again..."
  exit 1
fi
mkdir -p "$ID_SECRETS"
printf "%s" "$ID" >"$ID_SECRETS/pidgeon.id.pub"
cp "$ID_SECRETS/pidgeon.id.pub" "$TMP_SECRETS/pidgeon.id.pub"

mktmp() {
  local name

  name="$1"

  cp "$ID_SECRETS/$name" "$TMP_SECRETS/$name"
}

mkid() {
  local name
  local length
  local prefix
  local id

  name="$1"
  length="${2:-32}"
  prefix="${3:-}"

  id="$(openssl rand -base64 256 | tr -cd '[:alnum:]' | head -c "$length")"
  while [ "${#id}" -lt "$length" ]; do
    id="${id}x"
  done

  printf "%s" "${prefix:+$prefix-}$id" >"$ID_SECRETS/$name.id.pub"
}

mkkey() {
  local name
  local length
  local key

  name="$1"
  length="${2:-32}"

  key="$(openssl rand -base64 256 | tr -cd '[:alnum:]' | head -c "$length")"
  while [ "${#key}" -lt "$length" ]; do
    key="${key}x"
  done

  printf "%s" "$key" >"$ID_SECRETS/$name.key"
}

mkpass() {
  local name
  local length
  local passwd

  name="$1"
  length="${2:-32}"

  passwd="$(openssl rand -base64 256 | tr -cd '[:alnum:]' | head -c "$length")"
  while [ "${#passwd}" -lt "$length" ]; do
    passwd="${passwd}x"
  done

  printf "%s" "$passwd" >"$ID_SECRETS/$name.pass"
  printf "%s" "$(echo "$passwd" | mkpasswd --stdin)" >"$ID_SECRETS/$name.pass.pub"
}

mkpin() {
  local name
  local length
  local pin

  name="$1"
  length="${2:-4}"

  pin="$(openssl rand -hex 256 | tr -cd '[:digit:]' | head -c "$length")"
  while [ "${#pin}" -lt "$length" ]; do
    pin="${pin}0"
  done

  printf "%s" "$pin" >"$ID_SECRETS/$name.pin"
}

mkage() {
  local path

  name="$1"

  age-keygen -o "$ID_SECRETS/$name.age" 2>&1 |
    awk '{ print $3 }' >"$ID_SECRETS/$name.age.pub"
}

mkssh() {
  local name
  local comment

  if [[ "$2" == "" ]]; then
    name="$1"

    ssh-keygen -q -a 100 -t ed25519 -N "" \
      -f "$ID_SECRETS/$name.ssh"
  else
    name="$1"
    comment="$2"

    ssh-keygen -q -a 100 -t ed25519 -N "" \
      -C "$comment" \
      -f "$ID_SECRETS/$name.ssh"
  fi
}

mkssl() {
  local name
  local subj
  local ca

  if [[ "$3" == "" ]]; then
    name="$1"
    subj="$2"

    openssl genpkey -algorithm ED25519 \
      -out "$SECRETS/$name.ca" >/dev/null 2>&1
    openssl req -x509 \
      -key "$SECRETS/$name.ca" \
      -out "$SECRETS/$name.ca.pub" \
      -subj "/CN=$subj" \
      -days 3650 >/dev/null 2>&1
  else
    name="$1"
    subj="$2"
    ca="$3"

    openssl genpkey -algorithm ED25519 \
      -out "$ID_SECRETS/$name.crt" >/dev/null 2>&1
    openssl req -new \
      -key "$ID_SECRETS/$name.crt" \
      -out "$ID_SECRETS/$name.csr" \
      -subj "/CN=$subj" >/dev/null 2>&1
    if [[ -f "$ca.ca.srl" ]]; then
      openssl x509 -req \
        -in "$ID_SECRETS/$name.csr" \
        -CA "$ca.ca.pub" \
        -CAkey "$ca.ca" \
        -CAserial "$ca.ca.srl" \
        -out "$ID_SECRETS/$name.crt.pub" \
        -days 3650 >/dev/null 2>&1
    else
      openssl x509 -req \
        -in "$ID_SECRETS/$name.csr" \
        -CA "$ca.ca.pub" \
        -CAkey "$ca.ca" \
        -CAcreateserial \
        -out "$ID_SECRETS/$name.crt.pub" \
        -days 3650 >/dev/null 2>&1
    fi
  fi
}

mknebula() {
  local name
  local subj
  local ca
  local ip

  if [[ "$3" == "" ]]; then
    name="$1"
    subj="$2"

    nebula-cert ca \
      -name "$subj" \
      -duration 87600h \
      -out-crt "$SECRETS/$name.ca.pub" \
      -out-key "$SECRETS/$name.ca"
  else
    name="$1"
    subj="$2"
    ip="$3"
    ca="$4"

    nebula-cert sign \
      -name "$subj" \
      -ca-crt "$ca.ca.pub" \
      -ca-key "$ca.ca" \
      -ip "$ip" \
      -out-crt "$ID_SECRETS/$name.crt.pub" \
      -out-key "$ID_SECRETS/$name.crt"

    print "%s" "$ip" >"$ID_SECRETS/$name.ip.pub"
  fi
}

indent() {
  local text="$1"
  local amount="${2:-2}"

  local spaces
  spaces="$(printf "%${amount}s" "")"

  printf "%s\n" "$text" | sed "2,\$s/^/$spaces/"
}

mkpass "altibiz"
mktmp "altibiz.pass"
mkssh "altibiz" "altibiz"
mktmp "altibiz.ssh"
mktmp "altibiz.ssh.pub"

mkkey "api"
mktmp "api.key"

mkage "secrets"
mktmp "secrets.age"

if [[ ! -f "$SECRETS/postgres.ca" ]]; then
  mkssl "postgres" "ca"
fi
mkssl "postgres" "pidgeon-$ID" "$SECRETS/postgres"
mktmp "postgres.crt.pub"

mkkey "postgres-postgres"
mkkey "postgres-pidgeon"
mkkey "postgres-altibiz"
mktmp "postgres-altibiz.key"
cat >"$ID_SECRETS/postgres.sql" <<EOF
ALTER USER postgres WITH PASSWORD '$(cat "$ID_SECRETS/postgres-postgres.key")';
CREATE USER pidgeon PASSWORD '$(cat "$ID_SECRETS/postgres-pidgeon.key")';
CREATE USER altibiz PASSWORD '$(cat "$ID_SECRETS/postgres-altibiz.key")';

CREATE DATABASE pidgeon;
ALTER DATABASE pidgeon OWNER TO pidgeon;

\c pidgeon

GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO altibiz;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO altibiz;
GRANT ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA public TO altibiz;
EOF

api_key="$(cat "$ID_SECRETS/api.key")"
postgres_pidgeon_key="$(cat "$ID_SECRETS/postgres-pidgeon.key")"
postgres_pidgeon_key_url="$(echo "$postgres_pidgeon_key" | jq -Rr @uri)"
cat >"$ID_SECRETS/pidgeon.env" <<EOF
DATABASE_URL="postgres://pidgeon:$postgres_pidgeon_key_url@localhost/pidgeon?sslmode=disable"

PIDGEON_CLOUD_SSL="1"
PIDGEON_CLOUD_DOMAIN="$cloud_domain"
PIDGEON_CLOUD_API_KEY="$api_key"
PIDGEON_CLOUD_ID="pidgeon-$ID"

PIDGEON_DB_DOMAIN="localhost"
PIDGEON_DB_PORT="5433"
PIDGEON_DB_USER="pidgeon"
PIDGEON_DB_PASSWORD="$postgres_pidgeon_key"
PIDGEON_DB_NAME="pidgeon"

PIDGEON_NETWORK_IP_RANGE_START="$network_ip_range_start"
PIDGEON_NETWORK_IP_RANGE_END="$network_ip_range_end"
EOF

mkid "router-wifi" 16 "pidgeon"
mkkey "router-wifi" 32
mkkey "router-admin" 10
mkpin "router-wps"
mktmp "router-wifi.id.pub"
mktmp "router-wifi.key"
mktmp "router-admin.key"
mktmp "router-wps.pin"
cat >"$ID_SECRETS/wifi.env" <<EOF
WIFI_SSID="$(cat "$ID_SECRETS/router-wifi.id.pub")"
WIFI_PASS="$(cat "$ID_SECRETS/router-wifi.key")"
EOF

if [[ ! -f "$SECRETS/nebula.ca" ]]; then
  mknebula "nebula" "ca"
fi
mknebula "nebula" "pidgeon-$ID" "$vpn_ip" "$SECRETS/nebula"
mktmp "nebula.crt.pub"

cat >"$ID_SECRETS/secrets.yaml" <<EOF
altibiz.ssh.pub: |
  $(indent "$(cat "$ID_SECRETS/altibiz.ssh.pub")" 2)
altibiz.pass.pub: |
  $(indent "$(cat "$ID_SECRETS/altibiz.pass.pub")" 2)
pidgeon.env: |
  $(indent "$(cat "$ID_SECRETS/pidgeon.env")" 2)
postgres.crt: |
  $(indent "$(cat "$ID_SECRETS/postgres.crt")" 2)
postgres.crt.pub: |
  $(indent "$(cat "$ID_SECRETS/postgres.crt.pub")" 2)
postgres.sql: |
  $(indent "$(cat "$ID_SECRETS/postgres.sql")" 2)
wifi.env: |
  $(indent "$(cat "$ID_SECRETS/wifi.env")" 2)
nebula.ca.pub: |
  $(indent "$(cat "$SECRETS/nebula.ca.pub")" 2)
nebula.crt: |
  $(indent "$(cat "$ID_SECRETS/nebula.crt")" 2)
nebula.crt.pub: |
  $(indent "$(cat "$ID_SECRETS/nebula.crt.pub")" 2)
EOF

sops --encrypt \
  --age "$(
    printf "%s" \
      "$(cat "$ID_SECRETS/secrets.age.pub")"
  )" \
  "$ID_SECRETS/secrets.yaml" >"$ID_SECRETS/secrets.enc.yaml"
mktmp "secrets.enc.yaml"

mkdir -p "$ROOT/src/flake/host/pidgeon-$ID"
cp "$ID_SECRETS/secrets.enc.yaml" "$ROOT/src/flake/host/pidgeon-$ID/secrets.yaml"
