# Getting Started

## Dependencies

Add `es-fluent` and a runtime manager. Derive macros are enabled by default:

```toml
[dependencies]
es-fluent = "0.16"
unic-langid = "0.9"

# For simple apps and CLIs:
es-fluent-manager-embedded = "0.16"

# For Dioxus apps, enable only the runtime surface you use.
# es-fluent-manager-dioxus = { version = "0.7", features = ["client"] }
# es-fluent-manager-dioxus = { version = "0.7", features = ["ssr"] }
# es-fluent-manager-dioxus = { version = "0.7", features = ["client", "ssr"] }
```

## Project Configuration

For a new crate, let the CLI create the standard config and module scaffold:

```sh
cargo es-fluent init
```

This creates `i18n.toml`, `assets/locales/en/`, `src/i18n.rs`, and a
`pub mod i18n;` declaration in `src/lib.rs`. Existing `i18n` module
declarations must be `pub mod i18n;`, not private or `pub(crate)`. Pass
`--manager dioxus` or `--manager bevy` for framework-specific scaffolding, and
pass `--build-rs` when you want Cargo to rebuild automatically after locale file
changes.
`--build-rs` creates or updates `build.rs`; existing build-script logic is
preserved when `init` can add the tracking call to `fn main`. If an existing
`build.rs` has no updatable `fn main`, `init` reports the manual edit instead
of overwriting it, even with `--force`.
In a workspace, run `init` from the member crate or pass
`--path <member-crate>` or `--path <member-crate>/Cargo.toml`; virtual
workspace roots and manifests are rejected.
Plain `init` can scaffold missing es-fluent files in an existing Cargo package
directory, but it does not create `Cargo.toml`; the target must already have a
readable, parseable manifest with a `[package]` table.
By default `init` writes `i18n.rs` next to `src/lib.rs`; if `Cargo.toml`
declares a custom `[lib].path`, `init` uses that library file instead. The
chosen library path must remain inside the crate root, resolve to a source file
rather than a directory, must not use existing symlinked path components, and
must not itself be the generated `i18n.rs` module path.
Use `--locales "fr-FR, zh-CN"` to create additional non-fallback locale
directories up front; do not include the fallback locale in that list. Existing
locale directories that `init` reuses must be real directories, not symlinks.
Use `--namespaces "ui, errors"` to write a namespace allowlist, and
`--update-cargo-toml` to add the matching dependencies. When `--build-rs` is
also passed, the manifest update includes `es-fluent-build` under
`[build-dependencies]`. With `--manager dioxus --update-cargo-toml`,
`--dioxus-runtime client`, `--dioxus-runtime ssr`, or
`--dioxus-runtime "client, ssr"` selects the generated manager features;
omitting it enables both.
`--locales` must not include the fallback locale.

Before writing anything, `init` checks that the target is a Cargo package root,
then checks generated-file conflicts and directory targets.

`init` creates a library target because CLI inventory collection reads library
targets. Put derived message types in `src/lib.rs` or another library crate;
binary-only derived types in `src/main.rs` are not discovered by `generate`.

Or create an `i18n.toml` next to your crate's `Cargo.toml` manually:

```toml
# Default fallback language (required)
fallback_language = "en"

# Path to FTL assets relative to the crate root; must stay inside the crate
# and must not use existing symlinked path components or locale targets (required)
assets_dir = "assets/locales"

# Features to enable if the crate's es-fluent derives are gated behind a feature (optional)
fluent_feature = ["my-feature"]

# Optional allowlist of namespace values for FTL file splitting
namespaces = ["ui", "errors", "messages"]

# Optional: disable warnings when non-fallback messages copy fallback text
check_fallback_copies = false
```

The CLI and build tools use this file as the single source of truth for locating `.ftl` files and validating keys.
Locale directory names use canonical BCP-47 tags. Deprecated aliases such as
`iw` and `src` are rejected; use canonical replacements such as `he` and `sc`.
The executable README example ships `en`, `fr-FR`, and `zh-CN`, with `en` as
the fallback locale.

## End-to-End Example

Here's a minimal project that defines a localizable enum, generates the FTL skeleton, and prints a translated message.

### 1. Define a type

```rust
// src/lib.rs
use es_fluent::EsFluent;

#[derive(EsFluent)]
pub enum LoginError {
    InvalidPassword,
    UserNotFound { username: String },
}
```

### 2. Generate the FTL file

```sh
cargo es-fluent generate
```

This creates `assets/locales/en/{your_crate}.ftl` with a skeleton:

```ftl
## LoginError

login_error-InvalidPassword = Invalid Password
login_error-UserNotFound = User Not Found { $username }
```

Edit the values to your liking — the CLI will preserve your changes on subsequent runs.

### 3. Initialize and localize

```rust
// src/main.rs
use my_crate::i18n::I18n;
use unic_langid::langid;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let i18n = I18n::try_new_with_language(langid!("en"))?;

    let err = my_crate::LoginError::UserNotFound {
        username: "alice".to_string(),
    };
    println!("{}", i18n.localize_message(&err));
    // → "User Not Found alice"

    Ok(())
}
```

## Workflow Summary

A typical `es-fluent` workflow:

1. **Configure** — Create `i18n.toml` with your fallback language and asset path.
2. **Derive** — Annotate structs and enums with `#[derive(EsFluent)]`.
3. **Generate** — Run `cargo es-fluent generate` to create FTL file skeletons.
4. **Translate** — Edit the generated `.ftl` files with real translations.
5. **Use** — Call `i18n.localize_message(&value)` at runtime through an explicit manager.

When adding a new target language later, seed it from the fallback locale:

```sh
cargo es-fluent add-locale fr-FR
```

The fallback locale directory, such as `assets/locales/en`, must exist as a
directory before syncing or adding target locales; it may be empty before
generated keys exist. Existing target locale paths must be directories.
If the requested locale already exists and no fallback keys are missing,
rerunning `add-locale` is a successful no-op.
Locale creation preflights every selected crate for setup, fallback parse, and
requested-locale path errors before writing any selected crate. Unexpected
write-time I/O failures after preflight succeeds are still not rolled back.

Before committing, `cargo es-fluent status --all` summarizes generation,
formatting, sync, orphan-cleanup, and validation work that remains. It does not
edit project source or locale files, but it may prepare `.es-fluent` runner
metadata and Cargo build output while checking valid crates. If `.es-fluent`
already exists, it and existing entries below it must be real paths, not
symlinks. Empty selections and setup errors are reported before that runner
preparation.

The following chapters break down each of these steps in detail.
