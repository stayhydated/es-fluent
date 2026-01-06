# es-fluent-derive Architecture

This document details the architecture of the `es-fluent-derive` crate, which provides procedural macros for automating the registration of localizable types.

## Overview

`es-fluent-derive` is a procedural macro crate that inspects Rust structs and enums at compile time to:

1. Verify invalid or missing attributes (using `es-fluent-core::options`).
1. Generate `impl FluentDisplay` and `impl FluentValue` implementations for runtime usage.
1. Generate static registration code using `inventory::submit!`.

## Architecture

The crate acts as a compiler plugin that transforms Rust syntax trees into registration boilerplate.

```mermaid
flowchart TD
    subgraph INPUT["Input Code"]
        CODE["#[derive(EsFluent...)]<br/>struct/enum MyType { ... }"]
    end

    subgraph MACRO["Proc Macro"]
        PARSE["syn Parser"]
        OPTS["Options Extraction (es-fluent-core)"]
        GEN["Code Generaton"]
    end

    subgraph OUTPUT["Expanded Code"]
        IMPL["impl FluentDisplay"]
        VAL["impl From... for FluentValue"]
        INV["inventory::submit!"]
        STATIC[StaticFtlTypeInfo]
    end

    CODE --> PARSE
    PARSE --> OPTS
    OPTS --> GEN
    GEN --> IMPL
    GEN --> VAL
    GEN --> INV
    INV --> STATIC
```

## Macro Expansion logic

When `#[derive(EsFluent)]` is applied to a struct:

```rust
#[derive(EsFluent)]
#[fluent(message = "my-message")]
struct MyMessage {
    user: String,
}
```

The macro generates roughly:

```rust
// 1. Implementation of formatting traits
impl ::es_fluent::FluentDisplay for MyMessage {
    fn fluent_fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        // ... constructs args map ...
        write!(f, "{}", ::es_fluent::localize("my-message", Some(&args)))
    }
}

// 2. Helper for variable passing
impl From<&MyMessage> for ::es_fluent::FluentValue<'_> {
    fn from(value: &MyMessage) -> Self {
       // ...
    }
}

// 3. Inventory Registration (Hidden Module)
#[doc(hidden)]
mod __es_fluent_inventory_my_message {
    use super::*;
    
    // Static data constructed at compile time
    static VARIANTS: &[::es_fluent::__core::registry::StaticFtlVariant] = &[
        ::es_fluent::__core::registry::StaticFtlVariant {
             name: "MyMessage",
             ftl_key: "my-message",
             args: &["user"], 
             module_path: module_path!(),
        }
    ];

    static TYPE_INFO: ::es_fluent::__core::registry::StaticFtlTypeInfo = 
        ::es_fluent::__core::registry::StaticFtlTypeInfo {
            type_kind: ::es_fluent::__core::meta::TypeKind::Struct,
            type_name: "MyMessage",
            variants: VARIANTS,
            file_path: file!(),
            module_path: module_path!(),
        };

    // Submits pointer to .init_array section
    ::es_fluent::__inventory::submit!(&TYPE_INFO);
}
```

## Key Components

- **`darling`**: Used for declarative attribute parsing (`#[fluent(...)]`).
- **`es-fluent-core::options`**: Defines the target structures (`StructOpts`, `EnumOpts`) that attributes are parsed into.
- **`syn` / `quote`**: Standard tools for parsing and generating Rust code.
- **`es-fluent-core`**: Provides the runtime target types (`StaticFtlTypeInfo`) that the specific macro generates code for.

## Macros

All macros are designed to be orthogonal and independent. Code generation for one does not rely on another.

| Macro | Purpose | Code Generation Logic |
| :--- | :--- | :--- |
| `#[derive(EsFluent)]` | **Primary Messaging** | Generates a specific message ID for the struct (or one per enum variant). Implements `FluentDisplay` which calls `localize()` with those IDs. Registers `FtlTypeInfo` to inventory for FTL generation. |
| `#[derive(EsFluentChoice)]` | **Select Expressions** | Does *not* generate a message ID. Instead, implements `EsFluentChoice` so the type can be passed as a variable to *other* messages (e.g. `$gender ->`). The inventory registration marks it as a "Choice" type. |
| `#[derive(EsFluentKv)]` | **Key-Value Pairs** | Generates companion enums (e.g. `MyStructLabel`) where each variant corresponds to a field of the struct. Useful for form labels, placeholders, etc. |
| `#[derive(EsFluentThis)]` | **Self-Referencing** | Implements the `ThisFtl` trait. Registers the type's top-level name as a key (similar to how `EsFluentKv` registers fields). |
