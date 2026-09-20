# es-fluent-manager-dioxus

[![crates.io: es-fluent-manager-dioxus][crate-badge]][crate]

Typed localization for Dioxus `0.7.x`, with signal-backed client
context and request-scoped SSR.

Enable `client` for client rendering, `ssr` for server rendering, or both
when the application uses both runtime surfaces.

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

Configured packages call `es_fluent_build::track_i18n_assets()` from Cargo's
selected custom-build target. Derived fallback-locale messages are compile-time
checked by default. Set `missing_message_policy = "fallback-str"` in the owning
package's `i18n.toml` when client and request-scoped SSR lookup should return
snake_case field, variant, or type names after locale fallback is exhausted.

[crate-badge]: https://img.shields.io/crates/v/es-fluent-manager-dioxus.svg?label=es-fluent-manager-dioxus
[crate]: https://crates.io/crates/es-fluent-manager-dioxus
