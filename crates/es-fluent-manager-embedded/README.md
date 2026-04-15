[![Docs](https://docs.rs/es-fluent-manager-embedded/badge.svg)](https://docs.rs/es-fluent-manager-embedded/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-manager-embedded.svg)](https://crates.io/crates/es-fluent-manager-embedded)

# es-fluent-manager-embedded

A zero-setup, global localization manager for `es-fluent`.

This crate provides a "Just Works" experience for adding localization to standard Rust applications (CLIs, TUIs, desktop apps). It bundles your translations directly into the binary and provides a global singleton for access.

## Features

- **Embedded Assets**: Compiles your FTL files into the binary (using `rust-embed` under the hood).
- **Global Access**: Once initialized, you can call \`to_fluent_string() anywhere in your code without passing context around.
- **Thread Safe**: Uses `OnceLock` and atomic swaps for safe concurrent access.

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
use es_fluent::ToFluentString;
use unic_langid::langid;

fn main() {
    // 1. Initialize the global manager with the active language
    es_fluent_manager_embedded::init_with_language(langid!("en-US"));

    // 2. Localize things!
    let msg = MyMessage::Hello { name: "World" };
    println!("{}", msg.to_fluent_string());
}
```

If you prefer to initialize first and decide the locale later, `init()` and
`select_language(...)` remain available:

```rs
es_fluent_manager_embedded::init();
es_fluent_manager_embedded::select_language(langid!("fr"))
    .expect("manager initialized and locale is available");
```

`select_language(...)` returns an error if initialization was skipped or if no
discovered module can serve the requested locale.

If you want startup to fail on duplicate or invalid module registrations, use
the strict entry points:

```rs
es_fluent_manager_embedded::try_init_with_language(langid!("fr"))
    .expect("registry conflicts must be fixed before startup");
```

`try_init_with_language(...)` only publishes the singleton after the requested
language has been selected successfully.
