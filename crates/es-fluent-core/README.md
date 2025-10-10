# es-fluent-core

The `es-fluent-core` crate provides the foundational logic for the `es-fluent` ecosystem. It contains the core data structures, parsing logic, and analysis tools required to extract translation information from Rust source code decorated with `es-fluent` attributes.

This crate is primarily intended for internal use by other `es-fluent` crates, such as `es-fluent-derive` and `es-fluent-build`. It is generally not a crate that you would use directly in your application.

## Core Components

-   **Attribute Parsing**: Contains the logic for parsing and interpreting the `#[fluent(...)]` attributes on structs and enums.
-   **Code Analysis**: Provides functions to analyze Rust types and extract information relevant for generating Fluent translation keys (`FtlTypeInfo`).
-   **Data Structures**: Defines the core data structures, such as `FtlTypeInfo` and `FtlVariant`, which represent the extracted translation metadata.
-   **Error Handling**: Defines the error types used throughout the parsing and analysis process.
