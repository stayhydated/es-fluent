[![Docs](https://docs.rs/es-fluent-generate/badge.svg)](https://docs.rs/es-fluent-generate/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-generate.svg)](https://crates.io/crates/es-fluent-generate)

# es-fluent-generate

**Internal Crate**: Fluent file generation, merging, cleaning, and formatting logic.

This crate turns `FtlTypeInfo` inventories into deterministic `.ftl` files. It
powers the CLI's runner-backed `generate` and `clean` flows and also exposes the
formatting helpers used by the CLI's direct `format` command.

## What it does

- Generates fallback-locale `.ftl` files from registered Rust types
- Merges updates into existing files without discarding manual translations or
  comments in conservative mode
- Removes orphaned generated keys in aggressive and clean flows
- Prunes stale namespaced `.ftl` files during clean runs when a namespace no
  longer has any registered Rust types
- Splits output into namespaced files when type metadata requests it
- Sorts and normalizes Fluent AST output for reproducible diffs

## Who should use it

Most users should use [`es-fluent-cli`](../es-fluent-cli/README.md) instead.
Depend on `es-fluent-generate` directly only if you are building custom tooling
that needs the workspace's FTL merge and formatting behavior.
