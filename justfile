set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]

default:
    @just --list

fmt:
    cargo sort-derives
    cargo fmt
    taplo fmt
    bun run fmt

clippy:
    cargo clippy --workspace --all-features

check:
    cargo check --workspace --all-features

test:
    cargo test --workspace --all-features --all-targets

cov:
    cargo llvm-cov --workspace --all-features --all-targets

test-publish:
    cargo publish --workspace --dry-run --allow-dirty

test-docs:
    cargo doc --workspace --all-features --no-deps --open

ci: fmt check clippy test cov
    cargo machete

book:
    mdbook serve book

web:
    bun run build
    bun run dev
