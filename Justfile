default:
    @just --list

fmt:
    cargo sort-derives
    cargo fmt
    taplo fmt
    uv format

clippy:
    cargo clippy --workspace --all-features --exclude cosmic-example

test-publish:
  cargo publish --workspace --dry-run --allow-dirty
