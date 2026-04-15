# Getting Started

## Dependencies

Add `es-fluent` with `derive` support and a runtime manager:

```toml
[dependencies]
es-fluent = { version = "*", features = ["derive"] }
unic-langid = "*"

# For simple apps and CLIs:
es-fluent-manager-embedded = "*"
```

## Project Configuration

Create an `i18n.toml` next to your workspace's root `Cargo.toml`:

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

The CLI and build tools use this file as the single source of truth for locating `.ftl` files and validating keys.
Locale directory names use canonical BCP-47 tags such as `en-US`, `fr`, or
`de-DE-1901`.

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

This creates `assets/locales/en-US/{your_crate}.ftl` with a skeleton:

```ftl
## LoginError

login_error-InvalidPassword = Invalid Password
login_error-UserNotFound = User Not Found { $username }
```

Edit the values to your liking — the CLI will preserve your changes on subsequent runs.

### 3. Initialize and localize

```rust
// src/main.rs
use es_fluent::ToFluentString;
use unic_langid::langid;

// Register the embedded assets
es_fluent_manager_embedded::define_i18n_module!();

fn main() {
    es_fluent_manager_embedded::init_with_language(langid!("en-US"));

    let err = my_crate::LoginError::UserNotFound {
        username: "alice".to_string(),
    };
    println!("{}", err.to_fluent_string());
    // → "User Not Found alice"
}
```

## Workflow Summary

A typical `es-fluent` workflow:

1. **Configure** — Create `i18n.toml` with your fallback language and asset path.
2. **Derive** — Annotate structs and enums with `#[derive(EsFluent)]`.
3. **Generate** — Run `cargo es-fluent generate` to create FTL file skeletons.
4. **Translate** — Edit the generated `.ftl` files with real translations.
5. **Use** — Call `to_fluent_string()` at runtime through a manager.

The following chapters break down each of these steps in detail.
