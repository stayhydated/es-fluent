# AGENTS.md

This file is the working guide for contributors and coding agents in the `es-fluent` workspace.

Use it to answer three questions quickly:

1. Where does this documentation belong?
2. Which crates are public entry points versus integration points versus internals?
3. What other surfaces must be updated in the same change?

## Project summary

`es-fluent` is a Rust localization ecosystem built on top of [Project Fluent](https://projectfluent.org/).

Its priorities are:

1. **Type safety**: keep Rust code and translation files aligned at compile time.
2. **Ergonomics**: make common localization workflows require minimal boilerplate.
3. **Developer experience**: provide tooling that generates, validates, and keeps FTL files in sync.

For most application code, start with `crates/es-fluent`.

## Audience labels

These labels describe the crate or surface itself, not the documentation file you are editing:

- **User-facing**: Normal entry points for application developers.
- **Public integration**: Public crates meant for extensions, integrations, or deeper customization. These are not usually the default starting point.
- **Internal**: Workspace plumbing, implementation details, and maintenance tooling.

## Documentation rules

### User-facing documentation

These surfaces are always user-facing:

- every `README.md` in the workspace,
- the mdBook under `book/`.

Even for internal crates, `README.md` should explain:

- who the crate is for,
- what it does,
- what most users should use instead.

### Internal documentation

Only `docs/ARCHITECTURE.md` files are internal documentation.

Use them for:

- implementation details,
- subsystem boundaries,
- data flow,
- design rationale,
- internal relationships.

Do not put internal implementation detail into READMEs or the book.

## Synchronization rules

When changing a public workflow, public feature, or user-visible API shape:

1. Update the executable example in `examples/readme` when relevant.
2. Update the affected user-facing `README.md` files.
3. Update the matching `book/src/*.md` pages.
4. Keep these surfaces aligned in the same change unless there is a documented reason not to.

Additional rules:

- User-facing documentation should be example-first.
- Prefer a Rust snippet over prose-only explanations when showing behavior changes.
- `examples/readme` is the canonical source of truth for usage examples.

## Workspace map

### Main user-facing entry points

- `crates/es-fluent`
  Audience: **User-facing**
  Docs: [Architecture](crates/es-fluent/docs/ARCHITECTURE.md)
  Role: ecosystem facade, default entry point, and home of the registry types (`FtlTypeInfo`, `FtlVariant`, `RegisteredFtlType`).

- `crates/es-fluent-cli`
  Audience: **User-facing**
  Docs: [Architecture](crates/es-fluent-cli/docs/ARCHITECTURE.md)
  Role: primary CLI (`cargo es-fluent`) for validating and generating FTL files.

- `crates/es-fluent-manager-embedded`
  Audience: **User-facing**
  Docs: [Architecture](crates/es-fluent-manager-embedded/docs/ARCHITECTURE.md)
  Role: zero-setup backend for embedding FTL files in the binary.

- `crates/es-fluent-manager-bevy`
  Audience: **User-facing**
  Docs: [Architecture](crates/es-fluent-manager-bevy/docs/ARCHITECTURE.md)
  Role: Bevy integration for ECS and assets.

- `crates/es-fluent-lang`
  Audience: **User-facing**
  Docs: [Architecture](crates/es-fluent-lang/docs/ARCHITECTURE.md)
  Role: runtime language identification and localized language names for UI language pickers.

### Public integration crates

- `crates/es-fluent-derive`
  Audience: **Public integration**
  Docs: [Architecture](crates/es-fluent-derive/docs/ARCHITECTURE.md)
  Role: proc-macro crate for `#[derive(EsFluent)]`. Most users should depend on `es-fluent` instead of this crate directly.

- `crates/es-fluent-lang-macro`
  Audience: **Public integration**
  Docs: [Architecture](crates/es-fluent-lang-macro/docs/ARCHITECTURE.md)
  Role: generates type-safe language enums from asset folders. Most users should access this through `es-fluent-lang`.

- `crates/es-fluent-manager-core`
  Audience: **Public integration**
  Docs: [Architecture](crates/es-fluent-manager-core/docs/ARCHITECTURE.md)
  Role: abstract traits for localization backends (`I18nModule`, `Localizer`).

- `crates/es-fluent-manager-macros`
  Audience: **Public integration**
  Docs: [Architecture](crates/es-fluent-manager-macros/docs/ARCHITECTURE.md)
  Role: macros for asset discovery and module registration. Most users consume this indirectly through manager crates.

### Internal crates

- `crates/es-fluent-shared`
  Audience: **Internal**
  Docs: [Architecture](crates/es-fluent-shared/docs/ARCHITECTURE.md)
  Role: shared runtime-safe types, naming helpers, namespace rules, path utilities, and common errors.

- `crates/es-fluent-derive-core`
  Audience: **Internal**
  Docs: [Architecture](crates/es-fluent-derive-core/docs/ARCHITECTURE.md)
  Role: shared build-time derive logic, including option parsing and validation.

- `crates/es-fluent-toml`
  Audience: **Internal**
  Docs: [Architecture](crates/es-fluent-toml/docs/ARCHITECTURE.md)
  Role: `i18n.toml` parsing and path resolution shared by macros and CLI tooling.

- `crates/es-fluent-cli-helpers`
  Audience: **Internal**
  Docs: [Architecture](crates/es-fluent-cli-helpers/docs/ARCHITECTURE.md)
  Role: runtime logic executed inside the temporary runner crate.

- `crates/es-fluent-runner`
  Audience: **Internal**
  Docs: [Architecture](crates/es-fluent-runner/docs/ARCHITECTURE.md)
  Role: runner protocol types and `.es-fluent/metadata` path helpers.

- `crates/es-fluent-generate`
  Audience: **Internal**
  Docs: [Architecture](crates/es-fluent-generate/docs/ARCHITECTURE.md)
  Role: FTL AST manipulation, diffing, formatting, and merge behavior.

- `xtask`
  Audience: **Internal**
  Docs: [Architecture](xtask/docs/ARCHITECTURE.md)
  Role: maintenance task runner.

### Examples and web surfaces

- `examples/first-example`
  Minimal getting-started example using the embedded manager.

- `examples/thiserror-example`
  Demonstrates `thiserror` integration with localizable error types.

- `examples/example-shared-lib`
  Shared example library used by multiple examples.

- `examples/feature-gated-example`
  Demonstrates feature-gated `es-fluent` derives and configuration.

- `examples/bevy-example`
  Bevy integration example using `es-fluent-manager-bevy`.

- `examples/gpui-example`
  GPUI integration example using `es-fluent-manager-embedded`.

- `examples/readme`
  Canonical executable documentation examples. Keep this in sync with the root `README.md` and `book`.

- `web`
  Audience: **User-facing**
  Role: Astro-based GitHub Pages site hosting WASM demos and the mdBook.

- `book`
  Audience: **User-facing**
  Role: mdBook for public workflows and public crate usage.

## Working rules by change type

### When editing docs

- Keep READMEs and the book user-facing.
- Move implementation detail into `docs/ARCHITECTURE.md`.
- Prefer examples over prose-only explanations.
- Sync `examples/readme`, relevant READMEs, and book pages in the same change.

### When editing Rust crates

- Use `cargo` for build, test, and run tasks.
- Keep dependency versions in the workspace root `Cargo.toml`.
- Use `workspace = true` in member crates.
- Let each crate choose its own dependency features in its own `Cargo.toml`.
- Use `path` dependencies only in the root `Cargo.toml` and in examples.
- Non-example crates should reference workspace crates with `workspace = true`, not explicit paths.

### When writing tests

- Prefer [insta](https://insta.rs/) for snapshot tests when it fits better than assertion-heavy unit tests.
- Prefer raw multiline strings, or `quote! { ... }` in macro contexts, over escaped single-line literals for embedded Rust code.

### When editing JavaScript or web tooling

- Use [bun](https://bun.com/) for dependency management.
- Use [turborepo](https://turborepo.org/) as the build system.
