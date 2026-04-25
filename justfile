set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]

default:
    @just --list

fmt:
    cargo sort-derives
    cargo fmt
    cargo es-fluent-local format --all
    taplo fmt
    bun run fmt

clippy:
    cargo clippy --workspace --all-features

check:
    cargo check --workspace --all-features

test:
    cargo test --workspace --all-features --all-targets

dioxus-manager-feature-matrix:
    cargo test -p es-fluent-manager-dioxus --no-default-features
    cargo test -p es-fluent-manager-dioxus --no-default-features --features macros
    cargo test -p es-fluent-manager-dioxus --no-default-features --features desktop
    cargo test -p es-fluent-manager-dioxus --no-default-features --features web
    cargo test -p es-fluent-manager-dioxus --no-default-features --features mobile
    cargo test -p es-fluent-manager-dioxus --no-default-features --features ssr
    cargo test -p es-fluent-manager-dioxus --no-default-features --features desktop,ssr
    cargo test -p es-fluent-manager-dioxus --no-default-features --features desktop,ssr,macros
    cargo test -p es-fluent-manager-dioxus --all-features
    cargo clippy -p es-fluent-manager-dioxus --all-targets --all-features -- -D warnings

cov:
    cargo llvm-cov --workspace --all-features --all-targets

test-publish:
    cargo publish --workspace --dry-run --allow-dirty

test-docs:
    cargo doc --workspace --all-features --no-deps --open

ci: fmt check clippy test dioxus-manager-feature-matrix cov
    cargo machete

book:
    mdbook serve book

web:
    bun run build
    dx serve
