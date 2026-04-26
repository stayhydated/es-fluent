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

The client runtime is rooted by `use_init_i18n(...)` or `use_provide_i18n(...)`.
The hook stores `ManagedI18n` in Dioxus context and mirrors the requested language into a `Signal<LanguageIdentifier>` so render code subscribes to locale changes.
Hook initialization is one-shot because it is stored through `use_hook`; later prop changes do not replace the context owner.

`DioxusI18n` lookup methods always resolve through the `ManagedI18n` stored in the Dioxus context. Stale handles cannot route typed formatting through a newer global owner.

The client hook installs the `es-fluent` custom localizer bridge strictly:

- installing the same manager again is idempotent;
- installing a different client manager is rejected;
- installing over an SSR bridge is rejected;
- external replacement of the `es-fluent` custom localizer is rejected on the next Dioxus bridge operation.

The bridge stores the active client `Arc<FluentManager>` and compares same-owner checks with `Arc::ptr_eq`, not raw pointer IDs.

## SSR runtime

SSR has two lifecycles:

1. process startup installs `SsrI18nRuntime` and the global custom localizer bridge;
2. each request creates an `SsrI18n` with its own `ManagedI18n`.

`SsrI18n` construction does not mutate process-global state. During rendering, `SsrI18n` pushes its manager onto a thread-local stack, rebuilds or renders synchronously, then pops the manager on scope drop.

The SSR bridge callback reads the current manager from that stack. Missing messages are reported as handled misses with a warning. Calls outside an active request scope are also reported as handled misses with an error log, so incorrect SSR paths cannot silently fall through to another global localizer.

The thread-local scope is synchronous only. It must not cross `.await`, spawned tasks, streaming callbacks, or fullstack server boundaries.
