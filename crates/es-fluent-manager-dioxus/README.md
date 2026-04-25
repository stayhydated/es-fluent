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

For implementation details and lifecycle boundaries, see
[the architecture note](docs/ARCHITECTURE.md).

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

The default feature only enables `define_i18n_module!`; enable exactly one of
`web`, `desktop`, `mobile`, or `ssr` for runtime integration. Mixed client/SSR
feature sets are supported for examples and tests, but normal applications
should choose one bridge owner per process.

| Feature   | Runtime surface                                        |
| --------- | ------------------------------------------------------ |
| `desktop` | Client hooks for Dioxus desktop/mobile-style rendering |
| `web`     | Client hooks for Dioxus web rendering                  |
| `mobile`  | Client hooks for Dioxus mobile rendering               |
| `ssr`     | Synchronous request-scoped SSR rendering               |
| `macros`  | Re-exports `define_i18n_module!` only                  |

Enable the client feature that matches your renderer. Enabling multiple client
features is redundant but harmless because they all use the same hook runtime.

```toml
[dependencies]
dioxus = { version = "0.7", features = ["desktop"] }
es-fluent = { version = "0.15", features = ["derive"] }
es-fluent-manager-dioxus = { version = "0.7", features = ["desktop"] }
unic-langid = "0.9"
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
            "{i18n.localize_global_fluent(&UiMessage::Hello)}"
        }
    }
}
```

Prefer `localize_id(...)`, `try_localize_id(...)`, `localize_in_domain(...)`,
and `try_localize_in_domain(...)` when a lookup must go directly through the
current `DioxusI18n` context.

Use `i18n.localize_global_fluent(...)` or `use_global_localized(...)` inside
render code when you want locale changes to trigger rerenders for
`#[derive(EsFluent)]` values. These helpers read the Dioxus signal before
delegating to the process-global `ToFluentString` path, so they are reactive but
not context-bound if another owner later calls
`GlobalLocalizerMode::ReplaceExisting`. The explicit `global` name is
intentional. Plain `to_fluent_string()` still formats correctly after
initialization, but it does not subscribe the component to locale changes on its
own.

`initial_language` is read once for the lifetime of the root component. If a
parent prop should drive the locale after startup, call
`i18n.select_language(...)` from an event handler or effect. Selection is
best-effort by default: `requested_language()` records the requested UI
language, even when some modules do not support it and are skipped. Use
`select_language_strict(...)` when every discovered module must accept the
locale.

The rendering-friendly lookup helpers return the message id when a translation
is missing. Use `try_localize_id(...)`, `try_localize_in_domain(...)`, or the
matching `ManagedI18n` methods when tests or strict application code need to
distinguish a missing message from a translated value.

The `use_init_i18n(...)` and `use_provide_i18n(...)` helpers panic on setup
failure for concise examples. Applications that want to render an initialization
error can use `use_try_init_i18n(...)` or `use_try_provide_i18n_with_mode(...)`
and handle the returned `DioxusInitError`.

For production event handlers, prefer handling locale switch failures instead
of panicking:

```ignore
if let Err(error) = i18n.select_language(next) {
    eprintln!("locale switch failed: {error}");
}
```

Client hooks install an `es-fluent` process-global custom localizer so derived
types can keep using `to_fluent_string()`. Treat that bridge as a singleton:
the default `GlobalLocalizerMode::ErrorIfAlreadySet` mode rejects a second
distinct client owner and also rejects switching between the client and SSR
bridges. `GlobalLocalizerMode::ReuseIfSameOwner` is available for explicit
same-owner reuse, and `GlobalLocalizerMode::ReplaceExisting` is the only mode
that changes bridge ownership. Use replacement only in controlled examples,
tests, or single-owner applications. The bridge has no teardown/restore API;
tests and mixed client/SSR examples should run serially and use
`ReplaceExisting` deliberately when they need deterministic ownership.

Manual client setup must call
`ManagedI18n::install_client_global_localizer(...)`. Do not use a client bridge
for SSR. SSR uses `SsrI18n::install_global_localizer(...)` so `ToFluentString`
resolves through the synchronous request-scoped manager.

While the Dioxus bridge owns the global localizer, missing Dioxus messages fall
back to their message id instead of falling through to an unrelated global
`es-fluent` context. `ManagedI18n::manager()` is available as an integration
escape hatch, but using it to select languages bypasses the tracked
`requested_language()` value and Dioxus rerender signal.

## SSR

The `ssr` feature is separate because server-side rendering needs request-scoped
manager ownership rather than a client-side signal.

```toml
[dependencies]
dioxus = { version = "0.7", default-features = false, features = ["ssr"] }
es-fluent-manager-dioxus = { version = "0.7", features = ["ssr"] }
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

let html = i18n.rebuild_and_render(&mut vdom);
```

`SsrI18n` currently targets synchronous `dioxus::ssr` rendering helpers. It
does not yet wrap the higher-level `dioxus-server` fullstack router pipeline.
The default SSR constructor installs the thread-local global bridge
idempotently, so request-scoped `SsrI18n` values can be created repeatedly.
Applications may also call `SsrI18n::install_global_localizer(...)` once during
startup before constructing per-request managers.
Use `rebuild_and_render(...)` for the common path where localization can happen
during the Dioxus rebuild pass. The lower-level `render(&VirtualDom)` method
only scopes the final SSR serialization step and assumes the virtual DOM was
already rebuilt inside `with_sync_manager(...)`.

Do not hold `with_sync_manager(...)` scopes across `.await`, spawned tasks,
streaming render callbacks, or fullstack server boundaries. The manager scope is
thread-local and synchronous.

When client and SSR features are enabled in the same binary, only one bridge may
own the process-global custom localizer at a time. A second owner receives
`DioxusGlobalLocalizerError::OwnerConflict` unless it uses
`GlobalLocalizerMode::ReplaceExisting` deliberately.
