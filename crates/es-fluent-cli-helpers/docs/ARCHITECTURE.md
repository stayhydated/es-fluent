# es-fluent-cli-helpers Architecture

This document explains the role of `es-fluent-cli-helpers` in the `es-fluent` toolchain.

## Purpose

`es-fluent-cli-helpers` is a **runtime helper library** that runs inside the generated runner crate. It minimizes the Rust code needed in Jinja templates by providing well-tested functions for all command handlers.

## Architecture

```mermaid
flowchart TD
    subgraph GENERATED["Generated main.rs"]
        EXTERN["extern crate user_crate_a;<br/>extern crate user_crate_b;"]
        MAIN["fn main() {<br/>  es_fluent_cli_helpers::run();<br/>}"]
    end

    subgraph HELPERS["es-fluent-cli-helpers"]
        RUN["run()<br/>Parses args, dispatches"]

        subgraph COMMANDS["Command Handlers"]
            GEN["run_generate_with_options()"]
            CHK["run_check()"]
            CLN["run_clean_with_options()"]
        end

        subgraph MODULES["Internal Modules"]
            CLI["cli.rs<br/>Inventory collection"]
            GENMOD["generate.rs<br/>FTL generation"]
        end
    end

    EXTERN --> MAIN
    MAIN --> RUN
    RUN --> GEN & CHK & CLN
    GEN --> GENMOD
    CHK --> CLI
    CLN --> GENMOD
```

## Module Structure

```mermaid
classDiagram
    class lib {
        +run()
        +run_generate()
        +run_generate_with_options()
        +run_check()
        +run_clean_with_options()
    }

    class cli {
        +ExpectedKey
        +InventoryData
        +write_inventory_for_crate()
    }

    class generate {
        +EsFluentGenerator
        +FluentParseMode
        +GeneratorArgs
        +GeneratorError
    }

    lib --> cli : uses
    lib --> generate : uses
```

## Command Flow

```mermaid
sequenceDiagram
    participant Runner as es-fluent-runner
    participant Helpers as run()
    participant Inventory as es_fluent::registry
    participant Generator as EsFluentGenerator

    Runner->>Helpers: run()
    Helpers->>Helpers: Parse args (command, --crate, --mode, etc.)

    alt check command
        Helpers->>Inventory: get_all_ftl_type_infos()
        Inventory-->>Helpers: FtlTypeInfo[]
        Helpers->>Helpers: Filter by crate, build ExpectedKey[]
        Helpers-->>Runner: Write inventory.json
    else generate command
        Helpers->>Generator: EsFluentGenerator::builder()...build()
        Generator->>Generator: Collect inventory, generate FTL
        Generator-->>Helpers: changed: bool
        Helpers-->>Runner: Write result.json
    else clean command
        Helpers->>Generator: generator.clean()
        Generator-->>Helpers: changed: bool
        Helpers-->>Runner: Write result.json
    end
```
