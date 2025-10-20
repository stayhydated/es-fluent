default:
    @just --list

fmt:
    cargo sort-derives
    cargo fmt
    taplo fmt
    uv format

clippy:
    cargo clippy --workspace --all-features --exclude cosmic-example
