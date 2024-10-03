set windows-shell := ["nu.exe", "-c"]
set shell := ["nu", "-c"]

root := absolute_path('')
scripts := absolute_path('scripts')
cli := absolute_path('src/cli')
probe := absolute_path('src/probe')
assets := absolute_path('assets')
config := absolute_path('assets/config.toml')
artifacts := absolute_path('artifacts')
target := absolute_path('target')
docs := absolute_path('docs')

default: prepare

prepare:
  do -i { dvc install } o+e>| ignore
  dvc pull
  cd '{{probe}}'; poetry install --no-root
  docker compose down -v
  docker compose up -d
  sleep 3sec
  cd '{{cli}}'; cargo sqlx migrate run

lfs:
  glob '{{assets}}/*.{csv,sql}' | \
    each { |x| dvc add $x } | \
    ignore

ci:
  cd '{{probe}}'; poetry install --no-root

run *args:
  cd '{{cli}}'; cargo run -- --config '{{config}}' {{args}}

probe *args:
  cd '{{probe}}'; \
    $env.PIDGEON_PROBE_ENV = 'development'; \
    python -m probe.main {{args}}

format:
  cd '{{root}}'; cargo fmt --all
  yapf --recursive --in-place --parallel '{{probe}}'
  prettier --write '{{root}}'
  shfmt --write '{{root}}'

lint:
  prettier --check '{{root}}'
  cspell lint '{{root}}' --no-progress
  glob '{{scripts}}/*.sh' | each { |i| shellcheck $i } | str join "\n"
  ruff check '{{probe}}'
  cd '{{root}}'; cargo clippy -- -D warnings
  cd '{{probe}}'; pyright .

test:
  cd '{{root}}'; cargo test

build:
  rm -rf '{{artifacts}}'
  mkdir '{{artifacts}}'
  cd '{{root}}'; cargo build --release
  mv '{{target}}/release/pidgeon-cli' '{{artifacts}}/pidgeon'

docs:
  rm -rf '{{artifacts}}'
  mkdir '{{artifacts}}'
  cd '{{root}}'; cargo doc --no-deps
  cd '{{docs}}/en'; mdbook build
  cd '{{docs}}/hr'; mdbook build
  mv '{{target}}/doc' '{{artifacts}}/code'
  mv '{{docs}}/en/book' '{{artifacts}}/en'
  mv '{{docs}}/hr/book' '{{artifacts}}/hr'
  cp '{{docs}}/index.html' '{{artifacts}}'
