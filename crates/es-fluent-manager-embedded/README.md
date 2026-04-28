[![Docs](https://docs.rs/es-fluent-manager-embedded/badge.svg)](https://docs.rs/es-fluent-manager-embedded/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-manager-embedded.svg)](https://crates.io/crates/es-fluent-manager-embedded)

# es-fluent-manager-embedded

A zero-setup embedded localization manager for `es-fluent`.

This crate is for standard Rust applications such as CLIs, TUIs, and desktop apps. It bundles your translations directly into the binary and returns an explicit `EmbeddedI18n` handle for runtime lookup. Most framework applications should use their framework-specific manager instead.

## Features

- **Embedded Assets**: Compiles your FTL files into the binary.
- **Explicit Context**: Keep an `EmbeddedI18n` handle in application state and pass it to code that localizes messages.
- **Thread Safe**: Safe to clone and share after initialization.

## Quick Start

### 1. Define the Module

In your crate root (`lib.rs` or `main.rs`), tell the manager to scan your assets:

```rs
// a i18n.toml file must exist in the root of the crate
es_fluent_manager_embedded::define_i18n_module!();
```

### 2. Initialize & Use

In your application entry point:

```rs
use es_fluent::EsFluent;
use es_fluent_manager_embedded::EmbeddedI18n;
use unic_langid::langid;

#[derive(EsFluent)]
enum MyMessage {
    Hello { name: String },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let i18n = EmbeddedI18n::try_new_with_language(langid!("en-US"))?;

    let msg = MyMessage::Hello { name: "World".to_string() };
    println!("{}", i18n.localize_message(&msg));

    Ok(())
}
```

If you prefer to initialize first and decide the locale later, create the
context and call `select_language(...)` on that context:

```rs
let i18n = es_fluent_manager_embedded::EmbeddedI18n::try_new()?;
i18n.select_language(langid!("fr"))?;
```

`select_language(...)` returns an error if no discovered module can serve the
requested locale, or if a supported locale's resources would build a broken
Fluent bundle. When some modules support the requested locale and others do
not, the default switch keeps the supporting modules active. Failed switches
keep the previous ready locale active.
