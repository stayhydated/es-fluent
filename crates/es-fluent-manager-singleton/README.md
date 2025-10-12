# es-fluent-manager-singleton

The `es-fluent-manager-singleton` crate provides a convenient global singleton for the `FluentManager`. This is useful for applications that do not use a dependency injection framework and need a simple, globally accessible way to manage translations.

It is designed to work with embedded translations, using the `define_embedded_i18n_module!` macro to discover and compile translation files directly into the binary.

## Features

-   **Global `FluentManager`**: A `OnceLock`-guarded singleton that provides thread-safe, global access to the `FluentManager`.
-   **Simple Initialization**: A single `init()` function to discover all embedded i18n modules and initialize the manager.
-   **Easy Language Switching**: A `select_language()` function to change the active language for all registered modules.

## Usage

1.  In each of your crates that has translations, define a singleton-specific module:

```rs
// In my_crate/src/lib.rs
// This macro discovers languages from your `i18n` directory and registers
// the module for the embedded assets system.
es_fluent_manager_singleton::define_i18n_module!();
```

2.  At the start of your application, initialize the singleton:

```rs
// In main.rs
use unic_langid::langid;

// This macro discovers languages from your `i18n` directory and registers
// the module for the embedded assets system.
// In this case, for any EsFluent derived item included with your application's entrypoint.
es_fluent_manager_bevy::define_i18n_module!();

fn main() {
    es_fluent_manager_singleton::init();

    let lang_en = langid!("en-US");
    es_fluent_manager_singleton::select_language(&lang_en);
}
```

## Examples
- [gpui](../../examples/gpui-example)
- [iced](../../examples/iced-example)
