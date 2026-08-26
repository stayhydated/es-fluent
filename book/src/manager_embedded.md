# Embedded manager

Use `es-fluent-manager-embedded` for general Rust applications,
including CLIs, TUIs, desktop apps, and services. It compiles configured FTL
resources into the binary and returns a cloneable `EmbeddedI18n`
handle.

## Add the dependency

~~~toml
[dependencies]
es-fluent = "0.18"
es-fluent-manager-embedded = "0.18"
unic-langid = "0.9"
~~~

## Register package resources

Call the module macro from a library-reachable module:

~~~rust
// src/i18n.rs
pub use es_fluent_manager_embedded::EmbeddedI18n as I18n;

es_fluent_manager_embedded::define_i18n_module!();
~~~

~~~rust
// src/lib.rs
pub mod i18n;
~~~

In a workspace, call the macro in every package that owns FTL resources and
link those owner crates into the host. One manager discovers the linked
registrations; the host does not copy dependency FTL.

## Initialize and localize

~~~rust
use es_fluent::EsFluent;
use es_fluent_manager_embedded::EmbeddedI18n;
use unic_langid::langid;

#[derive(EsFluent)]
struct Greeting<'a> {
    name: &'a str,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let i18n = EmbeddedI18n::try_new_with_language(langid!("en"))?;
    let text = i18n.localize_message(&Greeting { name: "Ada" });
    println!("{text}");
    Ok(())
}
~~~

When the initial language is not known at construction time, call
`EmbeddedI18n::try_new()`, then
`select_language(...)` before typed lookup.

Typed `localize_message(...)` and
`localize_label(...)` treat a missing registered resource as a
configuration error. The fallback locale is compile-time checked when
`es-fluent-build` produces its catalog. Set
`missing_message_policy = "fallback-str"` in the owning package's `i18n.toml` to
return snake_case source names from normal typed lookup instead. Import
`es_fluent::FluentLocalizerExt as _` and use
`try_localize_message(...)` only at a boundary that handles the missing state.
Labels provide a matching `try_localize_label(...)`.

## Select languages

`select_language(...)` succeeds when at least one application module
can serve the requested locale and keeps supported modules active. Use
`select_language_strict(...)` or
`try_new_with_language_strict(...)` when every discovered
application module must support the locale.

A failed switch keeps the previous ready locale. Cloned
`EmbeddedI18n` handles share language state; construct a separate
manager when independent locale state is required.

WASM debug builds embed locale assets automatically. For other debug targets
that cannot read assets from the filesystem, enable the manager's
`debug-embed` feature.
