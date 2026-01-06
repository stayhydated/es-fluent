# es-fluent-cli Design

This document explains the architecture of `es-fluent-cli` and its relationship with `es-fluent-cli-helpers`.

## Overview

The CLI uses a **runner crate approach** to collect inventory registrations from user code at runtime. The CLI generates a persistent runner crate in `.es-fluent/` that links all workspace crates, then runs a binary that calls into `es-fluent-cli-helpers`.

## Architecture

```mermaid
flowchart TD
    subgraph USER["User Workspace"]
        UC[User Crates with EsFluent derives]
        I18N[i18n.toml configs]
    end

    subgraph CLI["es-fluent-cli"]
        CMD[Commands: generate, check, clean, watch]
        JINJA[Jinja Templates]
        CACHE[Caching Layer]
    end

    subgraph RUNNER[".es-fluent/ Runner Crate"]
        CARGO[Cargo.toml]
        MAIN[src/main.rs]
        BIN[es-fluent-runner binary]
    end

    subgraph HELPERS["es-fluent-cli-helpers"]
        RUN["run() entry point"]
        GEN["run_generate_with_options()"]
        CHK["run_check()"]
        CLN["run_clean_with_options()"]
    end

    subgraph OUTPUT["JSON Outputs"]
        INV[metadata/*/inventory.json]
        RES[metadata/*/result.json]
        HASH[metadata/*/content_hash.json]
    end

    CMD --> JINJA
    JINJA -->|generates| CARGO
    JINJA -->|generates| MAIN
    MAIN -->|calls| RUN
    UC -->|extern crate| BIN
    BIN --> RUN
    RUN --> GEN & CHK & CLN
    GEN & CHK & CLN --> INV & RES
    CACHE --> HASH
    CLI -->|reads| OUTPUT
```

## Jinja Templates

| Template | Output | Purpose |
|----------|--------|---------|
| `MonolithicCargo.toml.jinja` | `.es-fluent/Cargo.toml` | Dependencies linking all workspace crates |
| `monolithic_main.rs.jinja` | `.es-fluent/src/main.rs` | Entry point calling `es_fluent_cli_helpers::run()` |
| `config.toml.jinja` | `.es-fluent/.cargo/config.toml` | Cargo configuration for runner crate |

## Data Flow

```mermaid
sequenceDiagram
    participant User
    participant CLI as es-fluent-cli
    participant Runner as es-fluent-runner
    participant Helpers as es-fluent-cli-helpers

    User->>CLI: es-fluent generate
    CLI->>CLI: Check staleness (content hash)
    alt Runner is stale
        CLI->>Runner: cargo run
    else Runner is fresh
        CLI->>Runner: Direct binary execution
    end
    Runner->>Helpers: run()
    Helpers->>Helpers: Collect inventory
    Helpers->>Helpers: Generate FTL
    Helpers-->>CLI: Write result.json
    CLI->>CLI: Read result.json
    CLI->>User: Display results
```

## Caching

```mermaid
flowchart LR
    subgraph STALENESS["Staleness Detection"]
        SRC[Source .rs files]
        HASH[blake3 content hash]
        CACHE[runner_cache.json]
    end

    subgraph METADATA["Metadata Caching"]
        LOCK[Cargo.lock]
        META[cargo metadata call]
        MCACHE[metadata_cache.json]
    end

    SRC -->|per-crate hash| HASH
    HASH -->|compare| CACHE
    CACHE -->|fresh/stale| DECISION[Skip rebuild?]
    
    LOCK -->|blake3 hash| MCACHE
    MCACHE -->|cache hit| SKIP[Skip cargo metadata]
```

## Per-Crate Output Structure

```
.es-fluent/
├── Cargo.toml              # Generated from MonolithicCargo.toml.jinja
├── src/main.rs             # Generated from monolithic_main.rs.jinja
├── runner_cache.json       # Maps crate → content hash
├── metadata_cache.json     # Cached cargo_metadata results
└── metadata/
    └── {crate_name}/
        ├── inventory.json  # Expected keys + variables (from check)
        ├── result.json     # {"changed": bool} (from generate/clean)
        └── content_hash.json  # Per-crate blake3 hash
```
