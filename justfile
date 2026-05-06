set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]

default:
    @just --list

fmt:
    cargo sort-derives
    cargo fmt
    cargo es-fluent-local fmt --all
    bun run fmt
    taplo fmt
    rumdl fmt .

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
    cargo check -p es-fluent-manager-dioxus --target wasm32-unknown-unknown --no-default-features --features client

cov:
    cargo llvm-cov --workspace --exclude xtask --exclude web --all-features --all-targets

test-publish:
    cargo publish --workspace --dry-run --allow-dirty

test-docs:
    cargo doc --workspace --all-features --no-deps --open

ci: fmt check clippy test test-dioxus-manager-feature-matrix cov
    cargo machete

book:
    mdbook serve book

web-build:
    cargo xtask build bevy-demo
    cargo xtask build book
    cargo xtask build llms-txt
    cargo xtask build web

web: web-build
    dx serve --package web
