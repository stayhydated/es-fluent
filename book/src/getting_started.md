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

Create `i18n.toml` next to your crate's `Cargo.toml`, create the fallback
locale directory, and expose an i18n module from your library target when you
use manager macros:

```rust
// src/i18n.rs
pub use es_fluent_manager_embedded::{
    EmbeddedI18n as I18n, EmbeddedInitError, LocalizationError,
};

es_fluent_manager_embedded::define_i18n_module!();
```

```rust
// src/lib.rs
pub mod i18n;
```

Use the Dioxus or Bevy manager crate in that module for framework-specific
integrations. If manager macros scan locale assets at compile time, add
`es-fluent-build` under `[build-dependencies]` and call
`es_fluent_build::track_i18n_assets();` from `build.rs` so Cargo rebuilds when
locale files are added, removed, or renamed.

Create an `i18n.toml` next to your crate's `Cargo.toml`:

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
