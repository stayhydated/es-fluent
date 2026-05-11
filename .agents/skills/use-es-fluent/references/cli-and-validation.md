# CLI and Validation

Use this reference when changing project setup, generated FTL, docs, examples, or validation.

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
cargo es-fluent tree --all --variables
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

## Repository Documentation Sync

When changing a public workflow, feature, or user-visible API shape in this repository:

1. Update `examples/readme` when the workflow has executable usage.
2. Update affected user-facing `README.md` files.
3. Update matching `book/src/*.md` pages.
4. Keep examples first, using Rust snippets over prose-only explanations.
5. Put implementation detail in `docs/ARCHITECTURE.md`, not READMEs or the book.

`examples/readme` is the canonical source for usage examples. It should stay aligned with the root README and mdBook.

## Validation Choice

Run the narrowest command that proves the edited behavior:

- Skill-only edits: validate the skill folder with the skill validator.
- Derive macro behavior: run targeted tests for the affected derive crate, plus `cargo es-fluent generate/check` on an example if generated inventory changes.
- CLI behavior: run targeted `cargo test -p es-fluent-cli ...` or a focused CLI command against the affected fixture/example.
- Embedded manager usage: run `cargo check -p readme` or `cargo run -p readme` when example behavior changes.
- Dioxus manager usage: run the affected Dioxus crate/example check with the feature under discussion.
- Bevy manager usage: run the affected Bevy crate/example check.
- Docs-only edits: run the relevant docs, markdown, or mdBook check if available; otherwise document why validation was skipped.

Do not claim a workflow works unless a validation command, generated artifact, or equivalent check actually ran.
