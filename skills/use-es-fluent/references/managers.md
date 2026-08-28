# Runtime managers

Read this reference when wiring runtime localization or choosing an application
context.

## Embedded

Use for general Rust applications:

~~~toml
[dependencies]
es-fluent = "0.18"
es-fluent-manager-embedded = "0.18"
unic-langid = "0.9"
~~~

~~~rust
// src/i18n.rs
pub use es_fluent_manager_embedded::EmbeddedI18n as I18n;

es_fluent_manager_embedded::define_i18n_module!();
~~~

~~~rust
use es_fluent_manager_embedded::EmbeddedI18n;
use unic_langid::langid;

let i18n = EmbeddedI18n::try_new_with_language(langid!("en"))?;
let text = i18n.localize_message(&message);
~~~

Use `try_new_with_language_strict(...)` or
`select_language_strict(...)` only when every linked application
module must serve the locale. Clones share language state.

## Dioxus

Enable exactly the runtime surfaces used by the application:

~~~toml
[dependencies]
dioxus = "0.7"
es-fluent-manager-dioxus = { version = "0.7", features = ["client"] }

# SSR:
# es-fluent-manager-dioxus = { version = "0.7", features = ["ssr"] }

# Client and SSR:
# es-fluent-manager-dioxus = { version = "0.7", features = ["client", "ssr"] }
~~~

Register assets from a library module:

~~~rust
es_fluent_manager_dioxus::define_i18n_module!();
~~~

Client components receive context from
`DioxusAssetI18nProvider` and call `use_i18n()`.
The provider owns asynchronous asset loading and failure rendering.

SSR creates one shared `SsrI18nRuntime` and one
`SsrI18n` per request:

~~~rust
let runtime = SsrI18nRuntime::discovered();
let i18n = runtime.request(langid!("en")).await?;
~~~

Pass request state into the component tree. Enable both `client` and
`ssr` if SSR components use Dioxus hooks.

## Bevy

~~~toml
[dependencies]
bevy = "0.19"
es-fluent = "0.18"
es-fluent-manager-bevy = "0.19"
~~~

~~~rust
// src/i18n.rs
es_fluent_manager_bevy::define_i18n_module!();
~~~

Install `I18nPlugin`, derive `BevyFluentText` for values
used directly as `FluentText<T>`, and request `BevyI18n`
inside systems that localize directly.

~~~rust
App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(I18nPlugin::with_language(langid!("en")))
    .run();
~~~

Named fields marked `#[locale]` refresh from the requested locale and
must implement `TryFrom<&LanguageIdentifier>`. Use
`I18nSet` only when application systems need explicit ordering
around locale synchronization or text refresh.

## Lookup rules

- Use the concrete context's `localize_message(&value)` for typed
  messages.
- Pass the same context to `MyType::localize_label(&i18n)`.
- Import `es_fluent::FluentLocalizerExt as _` for fallible message
  lookup.
- Use `try_localize_message(...)` or
  `try_localize_label(...)` only when missing output is an expected
  state.
- Keep package-local strict fallback-locale compile validation as the default.
  Set `missing_message_policy = "fallback-str"` in the owning package's
  `i18n.toml` only when normal lookup should return snake_case source names after
  locale fallback fails. Embedded, Dioxus client/SSR, and Bevy use the policy
  carried by each generated key.
- Select a language before rendering. Failed switches keep the previous ready
  state.
