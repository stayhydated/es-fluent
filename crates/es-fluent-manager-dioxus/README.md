[![Docs](https://docs.rs/es-fluent-manager-dioxus/badge.svg)](https://docs.rs/es-fluent-manager-dioxus/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-manager-dioxus.svg)](https://crates.io/crates/es-fluent-manager-dioxus)

# es-fluent-manager-dioxus

Experimental [Dioxus](https://dioxuslabs.com/) integration for `es-fluent`.

Use this crate when a Dioxus app needs `es-fluent` module discovery plus a Dioxus-owned localization runtime. Most non-Dioxus applications should use [`es-fluent-manager-embedded`](../es-fluent-manager-embedded/README.md) or [`es-fluent-manager-bevy`](../es-fluent-manager-bevy/README.md) instead.

## Features

Choose the runtime surface explicitly:

```toml
# Client apps
es-fluent-manager-dioxus = { version = "0.7", features = ["client"] }

# Server-side rendering
es-fluent-manager-dioxus = { version = "0.7", features = ["ssr"] }
```

The crate has no default runtime feature. `define_i18n_module!` is always re-exported.

## Client

```toml
[dependencies]
dioxus = { version = "0.7", features = ["desktop"] }
es-fluent = { version = "0.15", features = ["derive"] }
es-fluent-manager-dioxus = { version = "0.7", features = ["client"] }
unic-langid = "0.9"
```

Register assets once from the crate with `i18n.toml`:

```ignore
es_fluent_manager_dioxus::define_i18n_module!();
```

Initialize the Dioxus runtime in the root component and localize through the returned context:

```ignore
use dioxus::prelude::*;
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
- `try_select_language(...)` records the requested language and updates the Dioxus signal used by render code.
- `try_select_language_strict(...)` requires every discovered module to support the requested locale.

The client runtime installs the `es-fluent` custom localizer bridge strictly and idempotently for the same Dioxus runtime. A different active client or SSR owner is rejected with `DioxusGlobalLocalizerError::OwnerConflict`; there is no public replacement mode.

## SSR

```toml
[dependencies]
dioxus = { version = "0.7", default-features = false, features = ["ssr"] }
es-fluent-manager-dioxus = { version = "0.7", features = ["ssr"] }
```

Install the SSR runtime once during startup, then create request-scoped `SsrI18n` values from it:

```ignore
use dioxus::prelude::*;
use es_fluent_manager_dioxus::ssr::SsrI18nRuntime;
use unic_langid::langid;

fn app() -> Element {
    rsx! { div { "hello" } }
}

let runtime = SsrI18nRuntime::install().expect("ssr runtime should install");
let i18n = runtime
    .request(langid!("en-US"))
    .expect("ssr i18n should initialize");

let mut vdom = VirtualDom::new(app);
let html = i18n.rebuild_and_render(&mut vdom);
```

`SsrI18n` scopes localization to the synchronous render call through a thread-local manager stack. Do not hold `with_sync_thread_local_manager(...)` across `.await`, spawned tasks, streaming callbacks, or fullstack server boundaries.

If SSR localization is called while the SSR bridge is installed but no request scope is active, the bridge logs and returns the message id. That makes incorrect render paths visible instead of silently falling through to unrelated global localization state.
