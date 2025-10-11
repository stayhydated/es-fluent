# es-fluent-manager-singleton

The `es-fluent-manager-singleton` crate provides a convenient global singleton for the `FluentManager`. This is useful for applications that do not use a dependency injection framework and need a simple, globally accessible way to manage translations.

It is designed to work with embedded translations, using the `define_embedded_i18n_module!` macro to discover and compile translation files directly into the binary.

## Features

-   **Global `FluentManager`**: A `OnceLock`-guarded singleton that provides thread-safe, global access to the `FluentManager`.
-   **Simple Initialization**: A single `init()` function to discover all embedded i18n modules and initialize the manager.
-   **Easy Language Switching**: A `select_language()` function to change the active language for all registered modules.

## Usage

1.  In each of your crates that has translations, define an embedded module:

    ```rs,no_run
    // In my_crate/src/lib.rs
    use es_fluent_manager_singleton::define_i18n_module;

    define_i18n_module!();
    ```

2.  At the start of your application, initialize the singleton:

    ```rs,no_run
    // In main.rs
    use es_fluent_manager_singleton::{init, select_language};
    use es_fluent::localize;
    use unic_langid::langid;

    fn main() {
        // Initializes the FluentManager with all discovered modules
        init();

        // Select the desired language
        let lang_en = langid!("en-US");
        select_language(&lang_en);

        // Now you can use the global `localize` function anywhere
        let greeting = localize("hello-world", None);
        println!("{}", greeting);
    }
    ```
