# es-fluent-manager-dioxus Architecture

This document describes the internal boundaries of the Dioxus manager crate.
User-facing setup lives in the crate README and the mdBook runtime manager page.

## Overview

The crate has three layers:

```text
ManagedI18n
  Safe language selection and lookup around FluentManager.

client
  Dioxus hooks and context-bound localization.

ssr
  One-time SSR bridge installation plus request-scoped rendering state.
```

The feature model mirrors those runtime boundaries:

- `client` enables `dioxus-core`, `dioxus-hooks`, and `dioxus-signals`.
- `ssr` enables `dioxus-core` and `dioxus-ssr`.
- `define_i18n_module!` is exported unconditionally.

## ManagedI18n

`ManagedI18n` owns an `Arc<FluentManager>` and a tracked requested language.
Public methods expose only safe operations:

- best-effort and strict language selection;
- requested language reads;
- direct message lookup;
- domain-scoped message lookup.

The raw manager is crate-private so public callers cannot change the manager's interior language state without updating the tracked requested language.

## Client runtime

The client runtime is rooted by `I18nProvider`, `use_init_i18n(...)`, or `use_provide_i18n(...)`.
The hook stores `ManagedI18n` in Dioxus context and mirrors the requested language into a `Signal<LanguageIdentifier>` so render code subscribes to locale changes.
Hook initialization is one-shot because it is stored through `use_hook`; later `initial_language`, provided manager, or `bridge_mode` changes do not replace the context owner or reinstall the bridge.

`DioxusI18n` lookup and language-selection methods always resolve through the `ManagedI18n` stored in the Dioxus context. The raw `ManagedI18n` is not exposed from `DioxusI18n`, and `ManagedI18n` is not publicly cloneable, so callers cannot retain a shared mutable handle that bypasses the signal update that makes language changes visible to render code. Typed `DioxusI18n::to_fluent_string_via_global_bridge(...)` reads the signal before delegating to `es-fluent`'s process-global `ToFluentString` path, so typed derived messages rerender after locale switches when the process-global bridge points at this Dioxus manager. It is not a direct context-bound lookup; `Disabled` bridge mode, `BestEffort` bridge failure, or external global-localizer replacement can make it disagree with the `DioxusI18n` context. `DioxusI18n::to_fluent_string(...)` remains as a deprecated alias with the same global semantics. `use_i18n_subscription()` and `try_use_i18n_subscription()` expose the same signal read as a hook-level subscription for components that want to keep direct `message.to_fluent_string()` call sites. The separate `es-fluent-manager-dioxus-derive` crate's `#[i18n_subscription]` attribute only expands to the optional subscription hook call.

The client hook installs the `es-fluent` custom localizer bridge strictly:

- installing the same manager again is idempotent;
- installing a different client manager is rejected;
- installing over an SSR bridge is rejected;
- external replacement of the `es-fluent` custom localizer is rejected on the next Dioxus bridge operation.

`DioxusClientBridgeMode` changes only the client bridge installation policy. `Strict` preserves the original behavior. `BestEffort` logs bridge installation failures and still provides context-bound lookup. `Disabled` skips bridge installation for apps that exclusively use explicit `DioxusI18n` lookup methods.

The bridge stores the active client `Arc<FluentManager>` and compares same-owner checks with `Arc::ptr_eq`, not raw pointer IDs.

Read-only diagnostics expose the current Dioxus owner and whether the recorded bridge still matches the active process-global custom localizer. There is intentionally no public reset API outside tests.

`ManagedI18n` implements `PartialEq` and `Eq` with the same identity model: values are equal only when they share the exact manager and requested-language state.

## SSR runtime

SSR has two lifecycles:

1. process startup installs `SsrI18nRuntime` and the global custom localizer bridge;
2. each request creates an `SsrI18n` with its own `ManagedI18n`.

`SsrI18nRuntime` is a non-default-constructible token returned by `SsrI18nRuntime::install`, so request state cannot be created through direct runtime construction. `SsrI18n` construction is private to `SsrI18nRuntime::request`, which revalidates the bridge before creating request state. Each request currently performs module discovery and creates a fresh `ManagedI18n`; a future high-throughput SSR optimization could cache immutable discovered module or bundle data in `SsrI18nRuntime` while keeping request-local language state separate. During rendering, `SsrI18n` revalidates bridge ownership, pushes its manager onto a thread-local stack, rebuilds or renders synchronously, then pops the manager on scope drop.

`SsrI18n::managed()` is public because the value is already request-scoped. This differs from the client runtime, where exposing the raw manager would let callers change language without updating the Dioxus signal.

The SSR bridge callback reads the current manager from that stack. Missing messages are reported as handled misses with a warning. Calls outside an active request scope are also reported as handled misses with an error log, so incorrect SSR paths cannot silently fall through to another global localizer.

The thread-local scope is synchronous only. It must not cross `.await`, spawned tasks, streaming callbacks, or fullstack server boundaries.
