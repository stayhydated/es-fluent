[![Docs](https://docs.rs/es-fluent-manager-dioxus/badge.svg)](https://docs.rs/es-fluent-manager-dioxus/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-manager-dioxus.svg)](https://crates.io/crates/es-fluent-manager-dioxus)

# es-fluent-manager-dioxus

Experimental [Dioxus](https://dioxuslabs.com/) integration for `es-fluent`.

This crate provides a Dioxus-oriented runtime layer on top of
`es-fluent-manager-core`:

- `web`, `desktop`, and `mobile` use the same embedded-asset discovery flow and
  expose hook-based locale management for reactive UI updates.
- `desktop` and `mobile` intentionally share the same client runtime because
  Dioxus 0.7 routes both through `dioxus-desktop`.
- `ssr` is separate and wraps synchronous `dioxus::ssr` rendering with a
  request-scoped localization bridge instead of a long-lived client context.

## Who Should Use It

Use this crate when your app is built with Dioxus and you want `es-fluent`
types to render inside Dioxus components without inventing a separate
localization store.

Most non-Dioxus applications should use a different manager instead:

- [`es-fluent-manager-embedded`](../es-fluent-manager-embedded/README.md) for
  CLIs, TUIs, and straightforward embedded-runtime apps
- [`es-fluent-manager-bevy`](../es-fluent-manager-bevy/README.md) for Bevy ECS
  and asset-driven apps

## Client Platforms

Enable the client feature that matches your renderer. Enabling multiple client
features is redundant but harmless because they all use the same hook runtime.

```toml
[dependencies]
dioxus = { version = "0.7", features = ["desktop"] }
es-fluent = { version = "*", features = ["derive"] }
es-fluent-manager-dioxus = { version = "*", features = ["desktop"] }
unic-langid = "*"
```

Register your locale assets once:

```ignore
es_fluent_manager_dioxus::define_i18n_module!();
```

Then initialize the Dioxus hook bridge in your root component:

```ignore
use dioxus::prelude::*;
use es_fluent::EsFluent;
use es_fluent_manager_dioxus::desktop::use_init_i18n;
use unic_langid::langid;

#[derive(EsFluent)]
enum UiMessage {
    Hello,
}

fn app() -> Element {
    let i18n = use_init_i18n(langid!("en-US"));

    rsx! {
        button {
            onclick: move |_| {
                let next = if i18n.peek_language() == langid!("en-US") {
                    langid!("fr")
                } else {
                    langid!("en-US")
                };
                i18n.select_language(next).expect("locale switch should succeed");
            },
            "{i18n.localize(&UiMessage::Hello)}"
        }
    }
}
```

Use `i18n.localize(...)` or `use_localized(...)` inside render code when you
want locale changes to trigger rerenders. Plain `to_fluent_string()` still
formats correctly after initialization, but it does not subscribe the component
to locale changes on its own.

Client hooks install an `es-fluent` process-global custom localizer so derived
types can keep using `to_fluent_string()`. Treat that bridge as a singleton:
do not run multiple Dioxus roots with different managers in the same process
unless one owner intentionally controls replacement. Use
`GlobalLocalizerMode::ReplaceExisting` only in controlled examples, tests, or
single-owner applications; libraries should keep the default
`ErrorIfAlreadySet` behavior.

## SSR

The `ssr` feature is separate because server-side rendering needs request-scoped
manager ownership rather than a client-side signal.

```toml
[dependencies]
dioxus = { version = "0.7", default-features = false, features = ["ssr"] }
es-fluent-manager-dioxus = { version = "*", features = ["ssr"] }
```

```ignore
use dioxus::prelude::*;
use es_fluent_manager_dioxus::ssr::SsrI18n;
use unic_langid::langid;

fn app() -> Element {
    rsx! { div { "hello" } }
}

let mut vdom = VirtualDom::new(app);
let i18n = SsrI18n::try_new_with_discovered_modules(langid!("en-US"))
    .expect("ssr i18n should initialize");

i18n.with_manager(|| {
    vdom.rebuild_in_place();
});

let html = i18n.render(&vdom);
```

`SsrI18n` currently targets synchronous `dioxus::ssr` rendering helpers. It
does not yet wrap the higher-level `dioxus-server` fullstack router pipeline.
The default SSR constructor installs the thread-local global bridge
idempotently, so request-scoped `SsrI18n` values can be created repeatedly.
Applications may also call `SsrI18n::install_global_localizer(...)` once during
startup before constructing per-request managers.
