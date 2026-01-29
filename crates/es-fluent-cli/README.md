[![Docs](https://docs.rs/es-fluent-cli/badge.svg)](https://docs.rs/es-fluent-cli/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-cli.svg)](https://crates.io/crates/es-fluent-cli)

# es-fluent-cli

The official command-line tool for `es-fluent`.

This tool automatically manages your Fluent (`.ftl`) translation files by analyzing your Rust code. It finds types with `#[derive(EsFluent)]` and creates corresponding message entries, so you don't have to keep them in sync manually.

## Installation

```sh
cargo install es-fluent-cli --locked
```

## Commands

Ensure you have an `i18n.toml` in your crate root:

```toml
# Default fallback language (required)
fallback_language = "en-US"

# Path to FTL assets relative to the config file (required)
assets_dir = "assets/locales"

# Features to enable if the crate’s es-fluent derives are gated behind a feature (optional)
fluent_feature = ["my-feature"]

# Optional allowlist of namespace values for FTL file splitting
namespaces = ["ui", "errors", "messages"]
```

### Generate

When you add new localizable structs or enums to your code, run:

```sh
cargo es-fluent generate
```

This will:

1. Scan your `src/` directory.
1. Update `i18n/en-US/{your_crate}.ftl` (and `i18n/en-US/{your_crate}/{namespace}.ftl` for namespaced types).
   - **New items**: Added as new messages.
   - **Changed items**: Variables updated (e.g. if you added a field).
   - **Existing translations**: Preserved untouched.

Use `--dry-run` to preview changes without writing them. Use `--force-run` to bypass the staleness cache and force a rebuild.

If you configure `namespaces = [...]` in `i18n.toml`, string-based namespaces are validated against the allowlist by both the compiler (at compile-time) and the CLI (during `generate` and `watch`).

### Namespaces (optional)

You can split output into multiple files by annotating types:

```rs
#[derive(EsFluent)]
#[fluent(namespace = "ui")] // -> assets_dir/{locale}/{crate}/ui.ftl
struct Button;

#[derive(EsFluent)]
#[fluent(namespace = file)] // -> assets_dir/{locale}/{crate}/{file_stem}.ftl
struct Dialog;

#[derive(EsFluent)]
#[fluent(namespace(file(relative)))] // -> assets_dir/{locale}/{crate}/ui/button.ftl
struct Modal;
```

### Watch

Same as `generate`, but runs in watch mode, updating FTL files as you type:

```sh
cargo es-fluent watch
```

### Check

To ensure all your translations are valid and no keys are missing:

```sh
cargo es-fluent check
```

Use `--all` to check all locales, not just the fallback language, `--ignore <CRATE>` to skip specific crates, `--force-run` to bypass the staleness cache.

### Clean

Remove orphan keys and groups that are no longer present in your source code:

```sh
cargo es-fluent clean
```

Use `--dry-run` to preview changes without writing them. Use `--all` to clean all locales. Use `--force-run` to bypass the staleness cache.

#### Clean Orphaned Files

Remove FTL files that are no longer tied to any registered types (e.g., when all items are now namespaced or the crate was deleted):

```sh
cargo es-fluent clean --orphaned --all
```

This compares files in non-fallback locales against the fallback locale (`en-US` by default). Files that exist in non-fallback locales but have no corresponding file in the fallback locale are considered orphaned and will be removed. The fallback locale itself is never modified.

Use `--dry-run` to preview which files would be removed without actually deleting them.

### Format

Standardize the formatting of your FTL files using `fluent-syntax` rules:

```sh
cargo es-fluent format
```

Use `--dry-run` to preview changes without writing them. Use `--all` to format all locales.

### Sync

Propagate keys from your fallback language to other languages (e.g., from `en-US` to `fr` and `de`), creating placeholders for missing translations:

```sh
cargo es-fluent sync
```

Use `--locale <LANG>` to sync a specific locale, or `--all` to sync all locales, `--dry-run` to preview changes without writing them.

The `sync` command properly handles namespaced FTL files, creating matching subdirectories in target locales when syncing from the fallback locale.

## Limitations

The CLI runner links workspace crates as **library targets only**. If you define
`#[derive(EsFluent*)]` types exclusively in a binary target, they won't be registered in the
inventory, and commands like `generate` or `clean` may miss or remove their keys.

Workarounds:

- Add a `lib.rs` target and move derived types into it.
- Move shared localization types into a small library crate and depend on it from your binary.
