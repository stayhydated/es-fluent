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

```rs
use dioxus::prelude::*;
use es_fluent_manager_dioxus::use_init_i18n;
use unic_langid::langid;

fn app() -> Element {
    let i18n = match use_init_i18n(langid!("en-US")) {
        Ok(i18n) => i18n,
        Err(error) => return rsx! { "Failed to initialize i18n: {error}" },
    };
    let label = match i18n.localize_in_domain(env!("CARGO_PKG_NAME"), "ui-hello", None) {
        Some(label) => label,
        None => return rsx! { "Missing message: ui-hello" },
    };

    rsx! {
        button {
            onclick: move |_| {
                if let Err(error) = i18n.select_language(langid!("fr")) {
                    eprintln!("locale switch failed: {error}");
                }
            },
            "{label}"
        }
    }
}
```

Use `use_provide_i18n(...)` when a caller has already constructed a `ManagedI18n`. Both hooks return `Result` so components can render initialization or bridge-installation failures. Hook initialization is one-shot; changing the initial language or provided manager after the first render does not replace the installed context.

After a `ManagedI18n` is handed to the Dioxus provider, route locale switches through `DioxusI18n::select_language(...)` or `DioxusI18n::select_language_strict(...)` so the Dioxus signal is updated with the manager state. `ManagedI18n` equality is identity equality over the shared manager and requested-language state, not semantic equality over modules or locale values.

Client localization is context-bound through `DioxusI18n`:

- `localize(...)` returns `Option<String>` from the current `ManagedI18n`.
- `localize_in_domain(...)` returns `Option<String>` from the current `ManagedI18n` plus an explicit domain.
- `localize_or_id(...)` and `localize_in_domain_or_id(...)` are explicit fallback helpers for UIs that intentionally render message IDs on misses.
- `requested_language()` returns the requested language, not necessarily the locale used by every message after fallback.
- `select_language(...)` records the requested language and updates the Dioxus signal used by render code.
- `select_language_strict(...)` requires every discovered module to support the requested locale.

Language-selection methods are fallible so UI event handlers can decide how to surface failures.

The client runtime installs the `es-fluent` custom localizer bridge automatically when a ready `ManagedI18n` context is provided. The bridge is strict:

- Reinstalling the same client manager is idempotent.
- A second distinct client owner is rejected with `DioxusGlobalLocalizerError::OwnerConflict`.
- SSR and client ownership conflict intentionally.
- External replacement of the global custom localizer is reported as `DioxusGlobalLocalizerError::ExternalReplacement`.
- There is no public bridge policy, replacement mode, disabled mode, or scoped bridge API.

If `use_init_i18n(...)` or `use_provide_i18n(...)` cannot initialize or install the client bridge, it still provides a failed context to keep hook order stable. Descendants can call `use_i18n_optional()` to distinguish a missing provider from a failed provider.

## SSR

```toml
[dependencies]
dioxus = { version = "0.7", default-features = false, features = ["ssr"] }
es-fluent-manager-dioxus = { version = "0.7", features = ["ssr"] }
```

Install the SSR runtime once during startup, then create request-scoped `SsrI18n` values from it:

```rs
use dioxus::prelude::*;
use es_fluent_manager_dioxus::ssr::SsrI18nRuntime;
use unic_langid::langid;

fn app() -> Element {
    rsx! { div { "hello" } }
}

fn render() -> Result<String, Box<dyn std::error::Error>> {
    let runtime = SsrI18nRuntime::install()?;
    let i18n = runtime.request(langid!("en-US"))?;

    let mut dom = VirtualDom::new(app);
    Ok(i18n.rebuild_and_render(&mut dom)?)
}
```

`SsrI18n` scopes localization to the synchronous render call through a thread-local manager stack. Render and scope methods revalidate that the SSR bridge still owns the global custom localizer before pushing request state. Do not hold `with_sync_thread_local_manager(...)` across `.await`, spawned tasks, streaming callbacks, or fullstack server boundaries.

`SsrI18n` values are constructed through `SsrI18nRuntime::request(...)`, which revalidates that the SSR bridge still owns the global custom localizer before creating request state.

If SSR localization is called while the SSR bridge is installed but no request scope is active, the bridge marks the lookup as missing and prevents fallthrough to unrelated global localization state. The string-returning `es-fluent` global helpers still render their normal message-id fallback.

## Process-global bridge lifecycle

The Dioxus client and SSR runtimes install an `es-fluent` custom localizer in process-global state. Treat that bridge as an application-lifetime resource:

- Install one Dioxus bridge owner per process.
- Client and SSR bridge ownership are mutually exclusive in the same process.
- Reinstalling the same client manager or SSR runtime is idempotent.
- Installing a different client manager, crossing client/SSR ownership, or replacing the global localizer externally is reported as an error.
- There is no public uninstall or reset API outside crate tests.
- Hot reload, test harnesses, and multi-root apps can hit these owner checks; handle the returned `Result` rather than assuming bridge installation always succeeds.
