# es-fluent-manager-core

The `es-fluent-manager-core` crate provides the central abstractions and runtime logic for managing different sources of translations in the `es-fluent` ecosystem. It defines the traits and structures for discovering, loading, and querying translation modules.

This crate is a foundational piece that enables both embedded and asset-based localization strategies.

## Core Concepts

-   **`I18nModule`**: A trait representing a discoverable unit of translations (e.g., a crate with its own `.ftl` files).
-   **`Localizer`**: A trait for an object that can load resources for a specific language and look up translations.
-   **`FluentManager`**: The main runtime struct that holds a collection of `Localizer` instances. It orchestrates language selection and message lookup across all registered modules.
-   **Module Types**:
    -   **Embedded**: For translations that are compiled directly into the binary using `rust_embed`.
    -   **Asset-based**: For translations that are loaded from external files at runtime, such as in a Bevy application.

This crate uses the `inventory` crate to automatically discover and register `I18nModule` instances at program startup, making the system highly modular.
