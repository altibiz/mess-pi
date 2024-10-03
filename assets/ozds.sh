#!/usr/bin/env bash

wait_for_server_and_restore() {
  export PGHOST=localhost
  export PGPORT=5432
  export PGDATABASE=ozds
  export PGUSER=ozds
  export PGPASSWORD=ozds

  echo "Ensuring postgres server is available..."
  until pg_isready; do
    echo "Waiting for postgres server to be available..."
    sleep 1s
  done

  echo "Ensuring timescaledb extension is available..."
  psql -c "CREATE EXTENSION IF NOT EXISTS timescaledb;"
  until psql -tAc "SELECT 1 FROM pg_proc WHERE proname = 'create_hypertable';" | grep -q 1; do
    echo "Waiting for create_hypertable function to be available..."
    sleep 1s
  done

  psql </assets/ozds.sql
}

wait_for_server_and_restore &

/docker-entrypoint.sh postgres
