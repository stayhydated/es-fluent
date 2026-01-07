[![Docs](https://docs.rs/es-fluent-cli/badge.svg)](https://docs.rs/es-fluent-cli/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-cli.svg)](https://crates.io/crates/es-fluent-cli)

# es-fluent-cli

The official command-line tool for `es-fluent`.

This tool automatically manages your Fluent (`.ftl`) translation files by analyzing your Rust code. It finds types with `#[derive(EsFluent)]` and creates corresponding message entries, so you don't have to keep them in sync manually.

## Installation

```sh
cargo install es-fluent-cli
```

## Commands

Ensure you have an `i18n.toml` in your crate root:

```toml
assets_dir = "i18n"
fallback_language = "en-US"
```

### Generate

When you add new localizable structs or enums to your code, run:

```sh
cargo es-fluent generate
```

This will:

1. Scan your `src/` directory.
1. Update `i18n/en-US/{your_crate}.ftl`.
   - **New items**: Added as new messages.
   - **Changed items**: Variables updated (e.g. if you added a field).
   - **Existing translations**: Preserved untouched.

Use `--dry-run` to preview changes without writing them.

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

Use `--all` to check all locales, not just the fallback language.

### Clean

Remove orphan keys and groups that are no longer present in your source code:

```sh
cargo es-fluent clean
```

Use `--dry-run` to preview changes without writing them.

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
