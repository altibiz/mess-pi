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

  name="$1"

  # NOTE: if you do it raw it adds a newline
  printf "%s" "$(openssl rand -base64 32)" >"$ID_SECRETS/$name.id.pub"
}

mkkey() {
  local name

  name="$1"

  # NOTE: if you do it raw it adds a newline
  printf "%s" "$(openssl rand -base64 32)" >"$ID_SECRETS/$name.key"
}

mkpass() {
  local name

  name="$1"

  # NOTE: if you do it raw it adds a newline
  passwd="$(openssl rand -base64 32)"
  printf "%s" "$passwd" >"$ID_SECRETS/$name.pass"
  printf "%s" "$(echo "$passwd" | mkpasswd --stdin)" >"$ID_SECRETS/$name.pass.pub"
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
  local ca
  local subj

  if [[ "$3" == "" ]]; then
    name="$1"
    subj="$2"

    openssl genpkey -algorithm ED25519 \
      -out "$SECRETS/$name.crt" >/dev/null 2>&1
    openssl req -x509 \
      -key "$SECRETS/$name.crt" \
      -out "$SECRETS/$name.crt.pub" \
      -subj "/CN=$subj" \
      -days 3650 >/dev/null 2>&1
  else
    name="$1"
    ca="$2"
    subj="$3"

    openssl genpkey -algorithm ED25519 \
      -out "$ID_SECRETS/$name.crt" >/dev/null 2>&1
    openssl req -new \
      -key "$ID_SECRETS/$name.crt" \
      -out "$ID_SECRETS/$name.csr" \
      -subj "/CN=$subj" >/dev/null 2>&1
    openssl x509 -req \
      -in "$ID_SECRETS/$name.csr" \
      -CA "$ca.crt.pub" \
      -CAkey "$ca.crt" \
      -CAcreateserial \
      -out "$ID_SECRETS/$name.crt.pub" \
      -days 3650 >/dev/null 2>&1
  fi
}

indent() {
  local text
  local amount

  text="$1"
  amount="$2"

  printf "%b" "$text" |
    sed -z "s/\\n/,/g;s/,/\\n$(printf "%${amount}s" "")/g"
}

mkpass "altibiz"
mktmp "altibiz.pass"
mkssh "altibiz" "altibiz"
mktmp "altibiz.ssh"
mktmp "altibiz.ssh.pub"

mkkey "api"
mktmp "api.key"

mkage "pidgeon"
mkage "altibiz"
mkage "secrets"
mktmp "secrets.age"

if [[ ! -f "$SECRETS/root.crt" ]]; then
  mkssl "root" "pidgeon root ca"
fi
mkssl "ca" "$SECRETS/root" "pidgeon-$ID ca"
mktmp "ca.crt.pub"
mkssl "postgres" "$ID_SECRETS/ca" "pidgeon-$ID postgres"
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

cat >"$ID_SECRETS/secrets.yaml" <<EOF
altibiz.pass.pub: |
  $(indent "$(cat "$ID_SECRETS/altibiz.pass.pub")" 2)
altibiz.ssh.pub: |
  $(indent "$(cat "$ID_SECRETS/altibiz.ssh.pub")" 2)
pidgeon.env: |
  $(indent "$(cat "$ID_SECRETS/pidgeon.env")" 2)
ca.crt: |
  $(indent "$(cat "$ID_SECRETS/ca.crt")" 2)
ca.crt.pub: |
  $(indent "$(cat "$ID_SECRETS/ca.crt.pub")" 2)
postgres.crt: |
  $(indent "$(cat "$ID_SECRETS/postgres.crt")" 2)
postgres.crt.pub: |
  $(indent "$(cat "$ID_SECRETS/postgres.crt.pub")" 2)
postgres.sql: |
  $(indent "$(cat "$ID_SECRETS/postgres.sql")" 2)
EOF

sops --encrypt \
  --age "$(
    printf "%s,%s,%s" \
      "$(cat "$ID_SECRETS/altibiz.age.pub")" \
      "$(cat "$ID_SECRETS/pidgeon.age.pub")" \
      "$(cat "$ID_SECRETS/secrets.age.pub")"
  )" \
  "$ID_SECRETS/secrets.yaml" >"$ID_SECRETS/secrets.enc.yaml"
mktmp "secrets.enc.yaml"

mkdir -p "$ROOT/src/flake/enc"
cp "$ID_SECRETS/secrets.enc.yaml" "$ROOT/src/flake/enc/$ID"
