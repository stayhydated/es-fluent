# es-fluent Architecture

`es-fluent` is the public facade for typed Fluent messages.

## Responsibilities

1. Re-export derive macros and shared metadata types.
2. Define the runtime traits used by managers:
   - `FluentMessage` for generated typed messages.
   - `FluentLocalizer` for explicit localization contexts.
   - `FluentLocalizerExt` for fallback helpers such as `localize_message(...)`.
   - `ThisFtl` for type-level Fluent keys rendered through an explicit context.
3. Re-export hidden inventory and asset dependencies needed by generated code.

## Runtime model

The crate exposes traits and metadata, but runtime localization always flows
through caller-provided contexts.

Derived messages call a caller-provided closure:

```rust
message.to_fluent_string_with(&mut |domain, id, args| {
    localizer.localize_in_domain_or_id(domain, id, args)
})
```

Managers implement `FluentLocalizer` and decide where state lives:

- `es-fluent-manager-embedded` stores state in an `EmbeddedI18n` handle.
- `es-fluent-manager-dioxus` stores state in Dioxus component/request context.
- `es-fluent-manager-bevy` stores state in Bevy resources.

## Generated-code contract

`#[derive(EsFluent)]` emits:

- `impl FluentMessage` for runtime rendering through explicit contexts.
- inventory metadata for generation/validation.

It does not emit a hidden display implementation or a conversion that performs
localization without context. Nested derived-message arguments are rendered with
the same explicit lookup closure as the outer message.
