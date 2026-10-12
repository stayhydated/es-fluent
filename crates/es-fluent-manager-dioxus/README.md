# es-fluent-manager-dioxus

[![crates.io: es-fluent-manager-dioxus][crate-badge]][crate]

Typed localization for Dioxus applications with signal-backed client state
and request-scoped SSR. Depend on this concrete manager alongside the
[`es-fluent`][facade] message facade, selecting the runtime features needed by
the application.

Requires Rust 1.99 or newer and Dioxus 0.7. Configured `assets_dir` must stay
inside the message-owning package root; its build script tracks fallback assets
and its library registers the Dioxus module.

## Overview

Client applications use `DioxusAssetI18nProvider` and handle the `Result` from
`use_i18n()` before localizing messages. The [maintained client demo][example]
shows provider setup, loading errors, typed lookup, and locale switching.
SSR applications share an `SsrI18nRuntime` and create one `SsrI18n` per request.
Both contexts resolve `i18n.localize_message(&message)` and follow the
[package-local missing-message policy][facade].

## Features

No runtime features are enabled by default.

| Feature | Default | Purpose |
| --- | --- | --- |
| `default` | Yes | Enables no features. |
| `client` | No | Enables Dioxus providers, signal-backed handles, and localization hooks. |
| `ssr` | No | Enables `SsrI18nRuntime` and request-scoped `SsrI18n` with asynchronous loading. |

Enable both `client` and `ssr` when SSR components use Dioxus hooks.

[facade]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent/README.md
[example]: https://github.com/stayhydated/es-fluent/blob/master/web/src/pages/dioxus_demo.rs
[crate-badge]: https://img.shields.io/crates/v/es-fluent-manager-dioxus.svg?label=es-fluent-manager-dioxus
[crate]: https://crates.io/crates/es-fluent-manager-dioxus
