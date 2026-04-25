[![Docs](https://docs.rs/es-fluent-manager-dioxus/badge.svg)](https://docs.rs/es-fluent-manager-dioxus/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-manager-dioxus.svg)](https://crates.io/crates/es-fluent-manager-dioxus)

# es-fluent-manager-dioxus

Experimental [Dioxus](https://dioxuslabs.com/) integration for `es-fluent`.

Use this crate when a Dioxus app needs `es-fluent` module discovery plus a Dioxus-owned localization runtime. Most non-Dioxus applications should use [`es-fluent-manager-embedded`](../es-fluent-manager-embedded/README.md) or [`es-fluent-manager-bevy`](../es-fluent-manager-bevy/README.md) instead.

## Features

Enable the runtime surfaces your crate actually uses:

```toml
# Client apps
es-fluent-manager-dioxus = { version = "0.7", features = ["client"] }

# Server-side rendering
es-fluent-manager-dioxus = { version = "0.7", features = ["ssr"] }
```

The crate has no default runtime feature. `define_i18n_module!` is always re-exported.

- `client`: Dioxus hook/context runtime for interactive rendering.
- `ssr`: synchronous Dioxus SSR runtime with request-scoped localization.

## Client

```toml
[dependencies]
dioxus = "0.7"
es-fluent = { version = "0.15", features = ["derive"] }
es-fluent-manager-dioxus = { version = "0.7", features = ["client"] }
unic-langid = "0.9"
```

Register assets once from the crate with `i18n.toml`:

```ignore
es_fluent_manager_dioxus::define_i18n_module!();
```

Initialize the Dioxus runtime in the root component and localize through the returned context:

```rust,no_run
use dioxus_core::Element;
use dioxus_core_macro::rsx;
#[allow(unused_imports)]
use dioxus_html as dioxus_elements;
use es_fluent_manager_dioxus::use_init_i18n;
use unic_langid::langid;

fn app() -> Element {
    let i18n = use_init_i18n(langid!("en-US"));
    let label = i18n.localize_in_domain(env!("CARGO_PKG_NAME"), "ui-hello", None);

    rsx! {
        button {
            onclick: move |_| {
                if let Err(error) = i18n.try_select_language(langid!("fr")) {
                    eprintln!("locale switch failed: {error}");
                }
            },
            "{label}"
        }
    }
}
```

Use `use_try_init_i18n(...)` when the component should render initialization errors instead of panicking. Use `use_provide_i18n(...)` or `use_try_provide_i18n(...)` when a caller has already constructed a `ManagedI18n`.

Client localization is context-bound through `DioxusI18n`:

- `localize(...)` and `try_localize(...)` use the current `ManagedI18n`.
- `localize_in_domain(...)` and `try_localize_in_domain(...)` use the current `ManagedI18n` plus an explicit domain.
- `requested_language()` returns the requested language, not necessarily the locale used by every message after fallback.
- `try_select_language(...)` records the requested language and updates the Dioxus signal used by render code.
- `try_select_language_strict(...)` requires every discovered module to support the requested locale.

Prefer the fallible language-selection methods in UI event handlers so render code can decide how to surface failures.

The client runtime installs the `es-fluent` custom localizer bridge automatically when a ready `ManagedI18n` context is provided. The bridge is strict:

- Reinstalling the same client manager is idempotent.
- A second distinct client owner is rejected with `DioxusGlobalLocalizerError::OwnerConflict`.
- SSR and client ownership conflict intentionally.
- There is no public bridge policy, replacement mode, disabled mode, or scoped bridge API.

If `use_try_init_i18n(...)` fails, it still provides a failed context to keep hook order stable, but `try_use_i18n()` returns `None` and no `DioxusI18n` is usable by children.

## SSR

```toml
[dependencies]
dioxus = { version = "0.7", default-features = false, features = ["ssr"] }
es-fluent-manager-dioxus = { version = "0.7", features = ["ssr"] }
```

Install the SSR runtime once during startup, then create request-scoped `SsrI18n` values from it:

```rust,no_run
use dioxus_core::{Element, VirtualDom};
use dioxus_core_macro::rsx;
#[allow(unused_imports)]
use dioxus_html as dioxus_elements;
use es_fluent_manager_dioxus::ssr::SsrI18nRuntime;
use unic_langid::langid;

fn app() -> Element {
    rsx! { div { "hello" } }
}

fn render() -> Result<String, Box<dyn std::error::Error>> {
    let runtime = SsrI18nRuntime::install()?;
    let i18n = runtime.request(langid!("en-US"))?;

    let mut dom = VirtualDom::new(app);
    Ok(i18n.rebuild_and_render(&mut dom))
}
```

`SsrI18n` scopes localization to the synchronous render call through a thread-local manager stack. Do not hold `with_sync_thread_local_manager(...)` across `.await`, spawned tasks, streaming callbacks, or fullstack server boundaries.

If SSR localization is called while the SSR bridge is installed but no request scope is active, the bridge logs and returns the message id. That makes incorrect render paths visible instead of silently falling through to unrelated global localization state.
