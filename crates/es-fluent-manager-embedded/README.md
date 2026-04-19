[![Docs](https://docs.rs/es-fluent-manager-embedded/badge.svg)](https://docs.rs/es-fluent-manager-embedded/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-manager-embedded.svg)](https://crates.io/crates/es-fluent-manager-embedded)

# es-fluent-manager-embedded

A zero-setup, global localization manager for `es-fluent`.

This crate provides a "Just Works" experience for adding localization to standard Rust applications (CLIs, TUIs, desktop apps). It bundles your translations directly into the binary and provides a global singleton for access.

## Features

- **Embedded Assets**: Compiles your FTL files into the binary.
- **Global Access**: Once initialized, you can call `to_fluent_string()` anywhere in your code without passing context around.
- **Thread Safe**: Safe to use from multiple threads after initialization.

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

`select_language(...)` returns an error if initialization was skipped, if no
discovered module can serve the requested locale, or if a supported locale's
resources would build a broken Fluent bundle (for example duplicate message
definitions across loaded files). When some modules support the requested
locale and others do not, the default switch keeps the supporting modules
active. Failed switches keep the previous ready locale active.

`init()` and `init_with_language(...)` use the same strict discovery path as
the fallible entry points. They log initialization errors instead of returning
them.

If you want the initialization error back before the singleton is published,
use the fallible entry points instead:

```rs
es_fluent_manager_embedded::try_init_with_language(langid!("fr"))
    .expect("embedded i18n manager should initialize");
```

Both `init_with_language(...)` and `try_init_with_language(...)` only publish
the singleton after the requested language has been selected successfully.
