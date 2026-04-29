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
use es_fluent::{EsFluent, EsFluentLabel};
use es_fluent_manager_embedded::EmbeddedI18n;
use unic_langid::langid;

#[derive(EsFluent, EsFluentLabel)]
#[fluent_label(origin)]
enum MyMessage {
    Hello { name: String },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let i18n = EmbeddedI18n::try_new_with_language(langid!("en"))?;

    let msg = MyMessage::Hello { name: "World".to_string() };
    println!("{}", i18n.localize_message(&msg));

    Ok(())
}
```

For types that derive `EsFluentLabel`, pass the same explicit context to
`localize_label(...)`:

```rs
use es_fluent::FluentLabel as _;

let title = MyMessage::localize_label(&i18n);
```

If you prefer to initialize first and decide the locale later, create the
context and call `select_language(...)` on that context:

```rs
let i18n = es_fluent_manager_embedded::EmbeddedI18n::try_new()?;
i18n.select_language(langid!("fr-FR"))?;
```

`select_language(...)` returns an error if no discovered module can serve the
requested locale, or if a supported locale's resources would build a broken
Fluent bundle. When some modules support the requested locale and others do
not, the default switch keeps the supporting modules active. Failed switches
keep the previous ready locale active.

When a locale has only some of a module's files, the available files can still
activate and missing messages fall back through the ICU4X locale fallback chain.
Utility modules such as localized language-name display follow successful
switches but do not make an otherwise unsupported locale count as supported.

Use `select_language_strict(...)` when every discovered module must support the
requested locale for the switch to succeed.

`EmbeddedI18n` clones are cheap shared handles. Calling
`select_language(...)` through one clone changes the active language observed
by the other clones. Construct a separate `EmbeddedI18n` value when you need
isolated language state.

`EmbeddedI18n` intentionally exposes enum-first `localize_message(...)` for application lookup. It also implements `FluentLocalizer` so generated labels and integration code can resolve through the same explicit context.
