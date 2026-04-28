# es-fluent-manager-dioxus architecture

`es-fluent-manager-dioxus` adapts the shared `FluentManager` runtime to Dioxus without using context-free localization hooks.

## Runtime surfaces

The crate has no default runtime feature.

- `client` enables provider and hook APIs for interactive Dioxus rendering.
- `ssr` enables request-scoped server-side rendering helpers.
- `define_i18n_module!` is always available for module registration.

## ManagedI18n

`ManagedI18n` owns an `Arc<FluentManager>` plus shared requested-language state. It is cloneable so SSR props and app-owned contexts can pass the same request manager through a component tree. Equality is identity equality over the shared manager and requested-language state.

Typed lookup is provided by `ManagedI18n::localize_message(...)`, which accepts `es_fluent::FluentMessage`. This keeps derive-generated message IDs and arguments while routing every lookup through the explicit manager.

## Client context

Client initialization is one-shot because it is stored through `use_hook`; later `initial_language` or provided-manager changes do not replace the context owner. `DioxusI18n` wraps the context handle and is the only client path that updates the signal used for rerendering after language changes.

`DioxusI18n::localize_message(...)` reads the tracked language signal before delegating to `ManagedI18n::localize_message(...)`. String-ID helpers follow the same signal-read pattern. Direct access to the raw `ManagedI18n` is intentionally not exposed from `DioxusI18n`, because direct client language changes would bypass the signal update that makes locale changes visible to render code.

Failed initialization is represented as a provided failed context. This keeps hook order stable and lets descendants distinguish a missing provider from a failed provider through `try_use_i18n()` or `use_i18n_optional()`.

## SSR runtime

`SsrI18nRuntime` caches strict discovered-module validation. `SsrI18nRuntime::request(...)` creates fresh `ManagedI18n` state from that cache for each request, keeping language selection isolated between requests.

SSR does not maintain a thread-local manager stack and does not install a context-free custom localizer. Components must receive a `ManagedI18n` explicitly, usually as a prop or through an app-owned context, and call `localize_message(...)` or explicit string-ID helpers.
