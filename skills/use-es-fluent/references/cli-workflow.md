# CLI Workflow

Use this reference when scaffolding projects, generating FTL, checking translations, formatting locale files, or keeping locale folders synchronized in Rust applications.

Examples use Cargo's subcommand form, `cargo es-fluent <COMMAND>`. The installed binary also accepts direct invocation as `cargo-es-fluent <COMMAND>`.

## Configuration

The standard `i18n.toml` lives next to the crate `Cargo.toml`:

```toml
fallback_language = "en"
assets_dir = "assets/locales"

# Optional: features needed to compile inventory for derives.
fluent_feature = ["my-feature"]

# Optional: restrict string namespace values.
namespaces = ["ui", "errors", "messages"]

# Optional: disable warnings when non-fallback messages copy fallback text.
check_fallback_copies = false
```

`assets_dir` is relative to the crate root. Locale directory names and locale arguments should use canonical BCP-47 tags such as `en`, `fr-FR`, and `zh-CN`.

## Setup

Create `i18n.toml` next to the crate `Cargo.toml`, create the fallback locale
directory, and put localizable types in a library target. Inventory collection
reads library targets, so binary-only derives in `src/main.rs` are not
discovered.

When using manager macros, expose a public i18n module from the library target
and call the manager crate's `define_i18n_module!()` macro from that module. If
locale assets are scanned at compile time, add `es-fluent-build` under
`[build-dependencies]` and call `es_fluent_build::track_i18n_assets();` from
`build.rs`.

## Routine Commands

## Routine Commands

After adding or changing derived localizable types:

```sh
cargo es-fluent generate
```

Generation updates fallback FTL, adds new messages, updates declared variables, and preserves existing translations in conservative mode.

Validate locale setup and Rust/FTL alignment:

```sh
cargo es-fluent check --all
```

With `--all`, check reports non-fallback messages that still match fallback text as untranslated warnings. For intentionally invariant text such as product names, package names, or keyboard keys, add this marker before that message:

```ftl
# es-fluent: same-as-fallback
```

Run a pre-commit status check:

```sh
cargo es-fluent status --all
```

Use `--all` when status should include non-fallback locale formatting, sync, orphan-file, and validation checks.

Format generated FTL:

```sh
cargo es-fluent fmt --all
```

Create missing non-fallback locale files from fallback files:

```sh
cargo es-fluent sync --all --create
```

Remove generated keys that no longer correspond to Rust derives:

```sh
cargo es-fluent clean --all
```

Inspect discovered locale files and Rust links:

```sh
cargo es-fluent tree
cargo es-fluent tree --output json
```

## Common Rules

Generated FTL keys must be unique within each output file. `generate`, `clean`, and `check` fail when two derived items produce the same key in the same output file.

For namespaced types, `check` validates the expected namespace file. A key in `{crate}.ftl` still counts as missing if the Rust type belongs in `{crate}/{namespace}.ftl`.

Comma-separated list options are trimmed, empty entries are rejected, and duplicate values are ignored in generated output.
