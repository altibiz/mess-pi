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
isready := absolute_path('scripts/isready.nu')
mksecrets-script := absolute_path('scripts/mksecrets.sh')
image-script := absolute_path('scripts/image.sh')
inject-script := absolute_path('scripts/inject.sh')
host-script := absolute_path('scripts/host.nu')

default:
    @just --choose

prepare:
    dvc pull
    docker compose down -v
    docker compose up -d
    {{ isready }}
    cd '{{ cli }}'; cargo sqlx migrate run

lfs:
    dvc add {{ assets }}/*.csv
    dvc add {{ assets }}/*.sql
    dvc push

run *args:
    cd '{{ cli }}'; cargo run -- --config '{{ config }}' {{ args }}

probe *args:
    cd '{{ probe }}'; \
      $env.PIDGEON_PROBE_ENV = 'development'; \
      python -m probe.main {{ args }}

format:
    cd '{{ root }}'; just --unstable --fmt
    cd '{{ root }}'; cargo fmt --all
    yapf --recursive --in-place --parallel '{{ probe }}'
    prettier --write '{{ root }}'
    shfmt --write '{{ root }}'

lint:
    cd '{{ root }}'; just --unstable --fmt --check
    prettier --check '{{ root }}'
    cspell lint '{{ root }}' --no-progress
    glob '{{ scripts }}/*.sh' | each { |i| shellcheck $i } | str join "\n"
    ruff check '{{ probe }}'
    cd '{{ root }}'; cargo clippy -- -D warnings
    cd '{{ probe }}'; pyright .

test:
    cd '{{ root }}'; cargo test

upgrade:
    nix flake update
    cargo upgrade

docs:
    rm -rf '{{ artifacts }}'
    mkdir '{{ artifacts }}'
    cd '{{ root }}'; cargo doc --no-deps
    cd '{{ docs }}/en'; mdbook build
    cd '{{ docs }}/hr'; mdbook build
    mv '{{ target }}/doc' '{{ artifacts }}/code'
    mv '{{ docs }}/en/book' '{{ artifacts }}/en'
    mv '{{ docs }}/hr/book' '{{ artifacts }}/hr'
    cp '{{ docs }}/index.html' '{{ artifacts }}'

rebuild *args:
    nixos-rebuild switch --flake $"{{ root }}#pidgeon-(open --raw /etc/id)-aarch64-linux" {{ args }}

host *args:
    {{ host-script }} {{ args }}

mksecrets *args:
    {{ mksecrets-script }} {{ args }}

image *args:
    {{ image-script }} {{ args }}

inject *args:
    {{ inject-script }} {{ args }}
