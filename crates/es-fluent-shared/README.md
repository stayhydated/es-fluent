# es-fluent-shared

**Internal Crate**: Runtime-safe types and helpers shared across the `es-fluent` workspace.

This crate exists so the public facade, generators, managers, and CLI tooling can
share the same metadata and filesystem helpers without depending on proc-macro-only
code.

## What it provides

- Registry metadata such as `FtlTypeInfo`, `FtlVariant`, and `TypeKind`
- Namespace and naming helpers such as `NamespaceRule`, `FluentKey`, and `FluentDoc`
- Shared error types: `EsFluentError` and `EsFluentResult`
- Path helpers for validating asset directories and parsing locale folder names

## Who should use it

Most applications should depend on [`es-fluent`](../es-fluent/README.md) instead.
Reach for `es-fluent-shared` directly only if you are building tooling or runtime
integrations around the workspace and need the shared metadata without pulling in
proc-macro-only code.
