# Introduction

`es-fluent` connects Rust types to
[Project Fluent](https://projectfluent.org/) messages. Derive macros define
typed message IDs and arguments, `cargo es-fluent` maintains Fluent
translation files, and runtime managers resolve those messages in embedded,
Dioxus, or Bevy applications.

This book is for Rust application developers who want to:

- generate `.ftl` resources from structs and enums;
- keep fallback and translated resources aligned;
- switch locales through an explicit runtime context;
- build typed language pickers; and
- validate localization in local development and CI.

Start with [Choose crates](workspace_map.md), then follow
[Getting started](getting_started.md) for a working embedded example. The
remaining chapters cover configuration, derive behavior, resource layout,
runtime integrations, and the CLI in more depth.

The examples assume familiarity with Cargo and basic Rust application
structure. Familiarity with Fluent syntax is useful when replacing generated
fallback text with production copy.
