# CLI Tooling

The `es-fluent-cli` is the command-line companion for `es-fluent`. It analyzes your Rust source code, finds types annotated with derive macros (see [Deriving Messages](deriving_messages.md)), and manages the corresponding FTL translation files for you.

## Installation

```sh
cargo install es-fluent-cli --locked
```

## Configuration

The CLI reads your `i18n.toml` (see [Getting Started](getting_started.md)) to locate FTL assets. Make sure it exists in your crate root:

```toml
# Default fallback language (required)
fallback_language = "en-US"

# Path to FTL assets relative to the config file (required)
assets_dir = "assets/locales"

# Features to enable if the crate's es-fluent derives are gated behind a feature (optional)
fluent_feature = ["my-feature"]

# Optional allowlist of namespace values for FTL file splitting
namespaces = ["ui", "errors", "messages"]
```

## Commands

### Generate

When you add new `#[derive(EsFluent)]`, `#[derive(EsFluentVariants)]`, or `#[derive(EsFluentThis)]` types to your code, run:

```sh
cargo es-fluent generate
```

This will:

1. Scan your `src/` directory for types with `es-fluent` derives.
1. Update `assets_dir/en-US/{your_crate}.ftl` (and `assets_dir/en-US/{your_crate}/{namespace}.ftl` for [namespaced](namespaces.md) types).
   - **New items**: Added as new messages.
   - **Changed items**: Variables updated (e.g. if you added a field).
   - **Existing translations**: Preserved untouched.

Use `--dry-run` to preview changes without writing them. Use `--force-run` to bypass the staleness cache and force a rebuild.

If you configure `namespaces = [...]` in `i18n.toml`, string-based namespaces are validated against the allowlist by both the compiler (at compile-time) and the CLI (during `generate` and `watch`).

### Watch

Same as `generate`, but runs in watch mode — updating FTL files as you type:

```sh
cargo es-fluent watch
```

### Check

Verify that all your translations are valid and no keys are missing:

```sh
cargo es-fluent check
```

Use `--all` to check all locales (not just the fallback language), `--ignore <CRATE>` to skip specific crates, `--force-run` to bypass the staleness cache.

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

The `sync` command properly handles [namespaced](namespaces.md) FTL files, creating matching subdirectories in target locales when syncing from the fallback locale.

## CI/CD Integration

### GitHub Actions

```yaml
name: es-fluent
on: [pull_request]

jobs:
  es-fluent:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - name: Check FTL files
        uses: stayhydated/es-fluent/crates/es-fluent-cli@master
        with:
          path: .
          all: true
```

Inputs:

- `version`: Version of `es-fluent-cli` to install from crates.io. Default: `latest`.
- `path`: Path to the crate or workspace root (passed as `--path`). Default: `.`.
- `package`: Package name filter for workspaces (passed as `--package`). Default: empty.
- `all`: Check all locales, not just the fallback language. Default: `false`.
- `ignore`: Crates to skip during validation (comma-separated). Default: empty.
- `force_run`: Force rebuild of the runner, ignoring the staleness cache. Default: `false`.
- `toolchain`: Rust toolchain to install for the action. Default: `stable`.

This action always runs `cargo es-fluent check`. Pin the `uses` ref to a release tag or commit SHA for reproducible builds. Use the `version` input to install a matching `es-fluent-cli` crate version.

## Limitations

The CLI runner links workspace crates as **library targets only**. If you define `#[derive(EsFluent*)]` types exclusively in a binary target, they won't be registered in the inventory, and commands like `generate` or `clean` may miss or remove their keys.

Workarounds:

- Add a `lib.rs` target and move derived types into it.
- Move shared localization types into a small library crate and depend on it from your binary.
