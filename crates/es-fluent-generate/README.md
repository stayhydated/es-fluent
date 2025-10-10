# es-fluent-generate

The `es-fluent-generate` crate is responsible for taking the structured translation metadata (`FtlTypeInfo`) extracted by `es-fluent-core` and generating `.ftl` (Fluent Translation List) files from it.

This crate is an internal component of the `es-fluent` ecosystem, used by tools like `es-fluent-build` and `es-fluent-cli` to perform the file generation step. You would not typically use this crate directly.

## Features

-   **FTL File Generation**: Creates `.ftl` files with messages and group comments based on the analyzed source code.
-   **Merging Logic**: Implements different strategies for merging new translation keys with existing `.ftl` files:
    -   **Conservative**: Preserves existing translations and comments, only adding new keys.
    -   **Aggressive**: Overwrites the file completely, which is useful for removing stale keys.
-   **Formatting**: Handles the basic formatting of the generated Fluent syntax.
