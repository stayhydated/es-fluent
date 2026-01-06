default:
    @just --list

fmt:
    cargo sort-derives
    cargo fmt
    taplo fmt
    uv format

clippy:
    cargo clippy --workspace --all-features

check:
    cargo check --workspace --all-features

test:
    cargo test --workspace --all-features

test-publish:
    cargo publish --workspace --dry-run --allow-dirty
