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

test-dioxus-manager-feature-matrix:
    cargo check -p es-fluent-manager-dioxus --no-default-features
    cargo test -p es-fluent-manager-dioxus --no-default-features --features client
    cargo test -p es-fluent-manager-dioxus --no-default-features --features ssr
    cargo test -p es-fluent-manager-dioxus --no-default-features --features client,ssr
    cargo check -p es-fluent-manager-dioxus --target wasm32-unknown-unknown --no-default-features --features client
    cargo check -p dioxus-client-example --target wasm32-unknown-unknown
    cargo check -p dioxus-client-example
    cargo check -p dioxus-ssr-example
    cargo test -p dioxus-client-example
    cargo test -p dioxus-ssr-example
    cargo test -p es-fluent-manager-dioxus --doc --no-default-features --features client,ssr
    cargo clippy -p es-fluent-manager-dioxus --no-default-features --features client,ssr --all-targets -- -D warnings

cov:
    cargo llvm-cov --workspace --all-features --all-targets

test-publish:
    cargo publish --workspace --dry-run --allow-dirty

test-docs:
    cargo doc --workspace --all-features --no-deps --open

ci: fmt check clippy test test-dioxus-manager-feature-matrix cov
    cargo machete

book:
    mdbook serve book

web:
    bun run build
    dx serve
