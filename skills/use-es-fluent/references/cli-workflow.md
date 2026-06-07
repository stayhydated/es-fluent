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

## Scaffolding

For a new crate, prefer:

```sh
cargo es-fluent init --update-cargo-toml
```

Useful options:

- `--manager dioxus` or `--manager bevy`: use a framework-specific module scaffold.
- `--dioxus-runtime client`, `--dioxus-runtime ssr`, or `--dioxus-runtime "client, ssr"`: choose generated Dioxus features; requires `--manager dioxus` and `--update-cargo-toml`.
- `--build-rs`: create or update `build.rs` with locale asset rebuild tracking for manager macros.
- `--fallback-language <LANG>`: choose fallback locale.
- `--locales fr-FR,zh-CN`: create additional non-fallback locale directories.
- `--assets-dir <PATH>`: choose locale asset directory relative to the crate root.
- `--namespaces ui,errors`: write a namespace allowlist.
- `--dry-run`: preview files and manifest updates without writing.
- `--force`: overwrite existing `i18n.toml` and i18n module scaffold targets when appropriate.

`init` creates or updates a library target because inventory collection reads library targets. If `Cargo.toml` declares a custom `[lib].path`, `init` uses that path and writes `i18n.rs` next to it.

If the library already defines an inline `mod i18n { ... }`, move that module to the generated `i18n.rs` path or remove it before running `init`. Existing external `i18n` declarations must be public; change `mod i18n;` or `pub(crate) mod i18n;` to `pub mod i18n;` before running `init`.

In a Cargo workspace, run `init` from the member crate or pass `--path <member-crate>` or `--path <member-crate>/Cargo.toml`.

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
