[![Docs](https://docs.rs/es-fluent-manager-dioxus/badge.svg)](https://docs.rs/es-fluent-manager-dioxus/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-manager-dioxus.svg)](https://crates.io/crates/es-fluent-manager-dioxus)

# es-fluent-manager-dioxus

Experimental [Dioxus](https://dioxuslabs.com/) integration for `es-fluent`.

This crate provides a Dioxus-oriented runtime layer on top of
`es-fluent-manager-core`:

- `client`, `web`, `desktop`, and `mobile` use the same embedded-asset discovery
  flow and expose hook-based locale management for reactive UI updates.
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

The default features enable `define_i18n_module!` and the generic `client` hook
runtime. Platform features are aliases for `client`; use them when they make
your Dioxus target clearer. Enable `ssr` for server rendering. Mixed client/SSR
feature sets are supported for examples and tests, but normal applications
should choose one bridge owner per process.

| Feature   | Runtime surface                                        |
| --------- | ------------------------------------------------------ |
| `client`  | Shared Dioxus client hook runtime                      |
| `desktop` | Client hooks for Dioxus desktop/mobile-style rendering |
| `web`     | Client hooks for Dioxus web rendering                  |
| `mobile`  | Client hooks for Dioxus mobile rendering               |
| `ssr`     | Synchronous request-scoped SSR rendering               |
| `macros`  | Re-exports `define_i18n_module!` only                  |

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

Then initialize the Dioxus provider in your root component:

```ignore
use dioxus::prelude::*;
use es_fluent::EsFluent;
use es_fluent_manager_dioxus::desktop::{
    GlobalBridgeLocalizationExt, GlobalBridgePolicy, I18nProviderConfig,
    use_i18n_provider_once,
};
use unic_langid::langid;

#[derive(EsFluent)]
enum UiMessage {
    Hello,
}

fn app() -> Element {
    let i18n = use_i18n_provider_once(
        I18nProviderConfig::new(langid!("en-US"))
            .with_global_bridge(GlobalBridgePolicy::ReplaceExisting),
    )
    .expect("i18n should initialize");

    rsx! {
        button {
            onclick: move |_| {
                let next = if i18n.peek_requested_language() == langid!("en-US") {
                    langid!("fr")
                } else {
                    langid!("en-US")
                };
                i18n.select_language(next).expect("locale switch should succeed");
            },
            "{i18n.localize_via_global(&UiMessage::Hello)}"
        }
    }
}
```

The public API has three layers:

1. Context-bound Dioxus localization through `DioxusI18n`; prefer this in
   components.
2. Synchronous request-scoped SSR localization through `SsrI18n`; prefer this
   during server rendering.
3. The process-global `ToFluentString` bridge; use it only for code paths that
   cannot receive an i18n context.

Prefer `localize_id(...)`, `try_localize_id(...)`, `localize_in_domain(...)`,
and `try_localize_in_domain(...)` when a lookup must go directly through the
current `DioxusI18n` context.

Use `GlobalBridgeLocalizationExt::localize_via_global(...)` or
`use_global_bridge_localized(...)` inside render code only when you explicitly
install a `GlobalBridgePolicy`. These helpers read the Dioxus signal before
delegating to the process-global `ToFluentString` path, so they are reactive but
not context-bound if another owner later calls
`GlobalBridgePolicy::ReplaceExisting`. Plain `to_fluent_string()` formats
through whatever process-global bridge is currently installed, but it does not
subscribe the component to locale changes on its own.

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

`use_i18n_provider_once(...)` and `use_provide_i18n_once(...)` return
`Result<DioxusI18n, DioxusInitError>` so applications can render or report
initialization failures. The provided `ManagedI18n` is a first-render value; do
not replace it through props. Call `select_language(...)` on the returned
`DioxusI18n` handle instead.

For production event handlers, prefer handling locale switch failures instead
of panicking:

```ignore
if let Err(error) = i18n.select_language(next) {
    eprintln!("locale switch failed: {error}");
}
```

Client hooks install an `es-fluent` process-global custom localizer so derived
types can keep using `to_fluent_string()`, but only when you opt in with
`GlobalBridgePolicy::InstallOnce` or `GlobalBridgePolicy::ReplaceExisting`.
`GlobalBridgePolicy::Disabled` is the context-only path. `InstallOnce` rejects a
distinct Dioxus owner and also rejects switching between the client and SSR
bridges. `ReplaceExisting` is the only policy that changes bridge ownership.
Use replacement only in controlled examples, tests, or single-owner
applications. The bridge has no teardown/restore API; tests and mixed client/SSR
examples should run serially and use `ReplaceExisting` deliberately when they
need deterministic ownership.

Manual client setup must call `ManagedI18n::install_client_global_bridge(...)`.
Do not use a client bridge for SSR. SSR uses `SsrI18n::install_global_bridge(...)`
so `ToFluentString` resolves through the synchronous request-scoped manager.

While the Dioxus bridge owns the global localizer, missing Dioxus messages fall
back to their message id instead of falling through to an unrelated global
`es-fluent` context. `ManagedI18n::raw_manager_untracked()` is available as an
integration escape hatch, but using it to select languages bypasses the tracked
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
Applications may also call `SsrI18n::install_global_bridge(...)` once during
startup before constructing per-request managers.
Use `rebuild_and_render(...)` for the common path where localization can happen
during the Dioxus rebuild pass. The lower-level `render(&VirtualDom)` method
only scopes the final SSR serialization step and assumes the virtual DOM was
already rebuilt inside `with_sync_manager(...)` or `with_scope(...)`.

Do not hold `with_sync_manager(...)` or `with_scope(...)` scopes across
`.await`, spawned tasks, streaming render callbacks, or fullstack server
boundaries. The manager scope is thread-local and synchronous. If SSR
localization is called outside an `SsrI18n` scope, the bridge returns the
message id instead of falling back to unrelated global localization state.

When client and SSR features are enabled in the same binary, only one bridge may
own the process-global custom localizer at a time. A second owner receives
`DioxusGlobalLocalizerError::OwnerConflict` unless it uses
`GlobalBridgePolicy::ReplaceExisting` deliberately.
