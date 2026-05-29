# CLI Workflow

Use this reference when scaffolding projects, generating FTL, checking translations, or maintaining generated locale files.

## Configuration

The standard `i18n.toml` lives next to the crate `Cargo.toml`:

```toml
fallback_language = "en"
assets_dir = "assets/locales"

# Optional: features needed to compile inventory for derives.
fluent_feature = ["my-feature"]

# Optional: restrict string namespace values.
namespaces = ["ui", "errors", "messages"]
```

Locale directory names should be canonical BCP-47 tags such as `en`, `fr-FR`, and `zh-CN`.

## Scaffolding

For a new crate, prefer:

```sh
cargo es-fluent init --update-cargo-toml
```

Useful options:

- `--manager dioxus` or `--manager bevy`: use a framework-specific module scaffold.
- `--dioxus-runtime client`, `--dioxus-runtime ssr`, or `--dioxus-runtime client,ssr`: choose generated Dioxus features.
- `--build-rs`: add locale asset rebuild tracking for manager macros.
- `--fallback-language <LANG>`: choose fallback locale.
- `--locales fr-FR,zh-CN`: create additional locale directories.
- `--assets-dir <PATH>`: choose locale asset directory relative to the crate root.
- `--namespaces ui,errors`: write a namespace allowlist.
- `--dry-run`: preview without writing.
- `--force`: overwrite generated files.

`init` creates a library target because inventory collection reads library targets.

## Routine Commands

After adding or changing derived localizable types:

```sh
cargo es-fluent generate
```

Generation updates fallback FTL, adds new messages, updates declared variables, and preserves existing translations in conservative mode.

Check translations and variables:

```sh
cargo es-fluent check --all
```

Run a read-only pre-commit status:

```sh
cargo es-fluent status --all
```

Format FTL:

```sh
cargo es-fluent fmt --all
```

Sync fallback keys to non-fallback locales:

```sh
cargo es-fluent sync --all
```

Add locale directories seeded from fallback:

```sh
cargo es-fluent add-locale fr-FR zh-CN
```

Remove stale generated entries:

```sh
cargo es-fluent clean --all
cargo es-fluent clean --orphaned --all
```

Inspect layout and message IDs:

```sh
cargo es-fluent tree --all
```

Diagnose setup:

```sh
cargo es-fluent doctor
```

All commands accept `--path <PATH>`/`-p <PATH>` and `--package <NAME>`/`-P <NAME>` when run from a workspace.

## Generated Layout

Without namespaces, generated fallback messages go to:

```text
assets_dir/{fallback_language}/{crate}.ftl
```

With namespaces:

```text
assets_dir/{fallback_language}/{crate}/{namespace}.ftl
```

Non-fallback locales mirror the fallback layout after `sync --all` or `add-locale`.
