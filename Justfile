default:
    @just --list

fmt:
    cargo sort-derives
    cargo fmt
    taplo fmt

clippy:
    cargo clippy --workspace --exclude cosmic-example
