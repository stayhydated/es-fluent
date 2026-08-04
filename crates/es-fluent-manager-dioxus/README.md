# es-fluent-manager-dioxus

[![Docs](https://docs.rs/es-fluent-manager-dioxus/badge.svg)](https://docs.rs/es-fluent-manager-dioxus/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-manager-dioxus.svg)](https://crates.io/crates/es-fluent-manager-dioxus)

Typed localization for Dioxus `0.7.x`, with signal-backed client
context and request-scoped SSR.

Choose the runtime surfaces the application uses:

~~~toml
[dependencies]
dioxus = "0.7"
es-fluent = "*"
es-fluent-manager-dioxus = { version = "*", features = ["client"] }

# SSR:
# es-fluent-manager-dioxus = { version = "*", features = ["ssr"] }

# Client and SSR:
# es-fluent-manager-dioxus = { version = "*", features = ["client", "ssr"] }
~~~

Register Dioxus assets from a library-reachable module:

~~~rust,ignore
es_fluent_manager_dioxus::define_i18n_module!();
~~~

Client applications provide `DioxusAssetI18nProvider` and localize
through the handle returned by `use_i18n()`. SSR applications create
one `SsrI18nRuntime` and one `SsrI18n` per request.

The configured `assets_dir` must be inside the package root. Enable
both `client` and `ssr` when SSR components use Dioxus
hooks.

See the [Dioxus manager guide](https://stayhydated.github.io/es-fluent/book/manager_dioxus.html)
for provider, locale switching, SSR, and asset-loading patterns.
