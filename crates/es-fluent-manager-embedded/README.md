# es-fluent-manager-embedded

A zero-setup, global localization manager for `es-fluent`.

This crate provides a "Just Works" experience for adding localization to standard Rust applications (CLIs, TUIs, desktop apps). It bundles your translations directly into the binary and provides a global singleton for access.

## Features

- **Embedded Assets**: Compiles your FTL files into the binary (using `rust-embed` under the hood).
- **Global Access**: Once initialized, you can call `to_fluent_string() anywhere in your code without passing context around.
- **Thread Safe**: Uses `OnceLock` and atomic swaps for safe concurrent access.

## Quick Start

### 1. Define the Module

In your crate root (`lib.rs` or `main.rs`), tell the manager to scan your assets:

```rust
// a i18n.toml file must exist in the root of the crate
es_fluent_manager_embedded::define_embedded_i18n_module!();
```

### 2. Initialize & Use

In your application entry point:

```rust
use es_fluent::ToFluentString;
use unic_langid::langid;

fn main() {
    // 1. Initialize the global manager
    es_fluent_manager_embedded::init();

    // 2. Set the language (e.g., from system locale or user config)
    es_fluent_manager_embedded::select_language(&langid!("en-US"));

    // 3. Localize things!
    let msg = MyMessage::Hello { name: "World" };
    println!("{}", msg.to_fluent_string());
}
```

## When to use

- **Use this if**: You are building a standalone app and want simplicity.
- **Don't use this if**: You are using Bevy (use `es-fluent-manager-bevy`) or need strictly decoupled, dependency-injected managers.
