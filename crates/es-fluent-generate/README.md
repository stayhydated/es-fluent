# es-fluent-generate

The `es-fluent-generate` crate is responsible for taking the structured translation metadata (`FtlTypeInfo`) extracted by `es-fluent-core` and generating `.ftl` (Fluent Translation List) files from it.

This crate is an internal component of the `es-fluent` ecosystem, used by `es-fluent-cli` or the `EsFluentGenerator` struct to perform the file generation step. You would not typically use this crate directly.
