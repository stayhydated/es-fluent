# es-fluent-manager-embedded

The `es-fluent-manager-embedded` crate provides a convenient global embedded singleton for the `FluentManager`. This is useful for applications that do not use a dependency injection framework and need a simple, globally accessible way to manage translations.

It is designed to work with embedded translations, using the `define_embedded_i18n_module!` macro to discover and compile translation files directly into the binary.

## Usage

1.  In each of your crates that has translations, define a embedded singleton-specific module:

```rs
// In my_crate/src/lib.rs
// This macro discovers languages from your `i18n` directory and registers
// the module for the embedded assets system.
es_fluent_manager_embedded::define_i18n_module!();
```

2.  At the start of your application, initialize the embedded singleton:

```rs
// In main.rs
use unic_langid::langid;

// This macro discovers languages from your `i18n` directory and registers
// the module for the embedded assets system.
// In this case, for any EsFluent derived item included with your application's entrypoint.
es_fluent_manager_bevy::define_i18n_module!();

fn main() {
    es_fluent_manager_embedded::init();

    let lang_en = langid!("en-US");
    es_fluent_manager_embedded::select_language(&lang_en);
}
```

## Examples
- [gpui](../../examples/gpui-example)
- [cosmic](../../examples/cosmic-example)
- [iced](../../examples/iced-example)
