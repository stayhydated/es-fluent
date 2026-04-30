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
```

## Project Configuration

For a new crate, let the CLI create the standard config and module scaffold:

```sh
cargo es-fluent init
```

This creates `i18n.toml`, `assets/locales/en/`, `src/i18n.rs`, and a
`pub mod i18n;` declaration in `src/lib.rs`. Pass `--manager dioxus` or
`--manager bevy` for framework-specific scaffolding, and pass `--build-rs`
when you want Cargo to rebuild automatically after locale file changes.
Use `--locales fr-FR,zh-CN` to create additional locale directories up front,
`--namespaces ui,errors` to write a namespace allowlist, and
`--update-cargo-toml` to add the matching dependencies. For Dioxus manifests,
`--dioxus-runtime client`, `--dioxus-runtime ssr`, or
`--dioxus-runtime client,ssr` selects the generated manager features; omitting
it enables both.

Before writing anything, `init` checks generated-file conflicts, directory
targets, and `Cargo.toml` parseability when manifest updates are requested.

`init` creates a library target because CLI inventory collection reads library
targets. Put derived message types in `src/lib.rs` or another library crate;
binary-only derived types in `src/main.rs` are not discovered by `generate`.

Or create an `i18n.toml` next to your crate's `Cargo.toml` manually:

```toml
# Default fallback language (required)
fallback_language = "en"

# Path to FTL assets relative to the config file (required)
assets_dir = "assets/locales"

# Features to enable if the crate's es-fluent derives are gated behind a feature (optional)
fluent_feature = ["my-feature"]

# Optional allowlist of namespace values for FTL file splitting
namespaces = ["ui", "errors", "messages"]
```

The CLI and build tools use this file as the single source of truth for locating `.ftl` files and validating keys.
Locale directory names use canonical BCP-47 tags. The executable README example
ships `en`, `fr-FR`, and `zh-CN`, with `en` as the fallback locale.

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

Before committing, `cargo es-fluent status --all` gives a read-only summary of
generation, formatting, sync, orphan-cleanup, and validation work that remains.

The following chapters break down each of these steps in detail.
