# AGENTS.md

This is the working guide for contributors and coding agents in the `es-fluent`
workspace.

Use it to decide:

1. where documentation belongs,
2. whether a crate or surface is user-facing, public integration, or internal,
3. which related docs, examples, and skills must change together,
4. which validation command should run before handoff.

For most application code, start with `crates/es-fluent`.

## Project Summary

`es-fluent` is a Rust localization ecosystem built on top of
[Project Fluent](https://projectfluent.org/).

Its priorities are:

1. **Type safety**: keep Rust code and translation files aligned at compile time.
2. **Ergonomics**: make common localization workflows require minimal boilerplate.
3. **Developer experience**: provide tooling that generates, validates, and keeps FTL files in sync.

## Quick Decision Flow

Before editing, classify the change:

1. **Find the surface in the workspace map.** Use its audience label to decide
   how much public explanation the change needs.
2. **Place documentation by content, not by crate audience.** README files and
   the book are always user-facing. Internal design belongs in the matching
   `docs/ARCHITECTURE.md`.
3. **Sync public workflow changes.** If behavior, commands, generated output,
   or recommended usage changes, update the relevant example, README, book page,
   and `.agents/skills/*` guidance in the same change when applicable.
4. **Validate narrowly.** Run the smallest command that proves the edited
   behavior or documentation surface is still sound.

## Audience Labels

These labels describe the crate or surface itself, not the documentation file
being edited:

- **User-facing**: normal entry points for application developers.
- **Public integration**: public crates meant for extensions, integrations, or
  deeper customization. These are usually not the default starting point.
- **Internal**: workspace plumbing, implementation details, and maintenance tooling.

## Documentation Placement

### User-Facing Documentation

Treat these surfaces as user-facing:

- every `README.md` in the workspace,
- the mdBook under `book/`.

Even README files for internal crates should explain:

- who the crate is for,
- what it does,
- what most users should use instead.

Keep user-facing documentation example-first. Prefer Rust snippets over
prose-only explanations when showing behavior changes.

### Internal Documentation

Use the relevant `docs/ARCHITECTURE.md` file for internal documentation, such
as the crate-level paths listed in the workspace map.

Keep these topics in architecture documents, not in READMEs or the book:

- implementation details,
- subsystem boundaries,
- data flow,
- design rationale,
- internal relationships.

### Skill Guidance

`.agents/skills/use-es-fluent` is hosted in this repository as public
`es-fluent` usage guidance for application developers. It is not internal
architecture, maintenance, CI, release, or contributor-only workflow
documentation.

Update relevant in-repository `.agents/skills/*` guidance when a code change
alters user-facing workflows, CLI behavior, generated output, integration
patterns, or recommended usage.

## Synchronization Rules

When a substantive change modifies a public workflow, public feature, or
user-visible API shape:

1. Update the executable example in `examples/readme` when relevant.
2. Update the affected user-facing `README.md` files.
3. Update the matching `book/src/*.md` pages.
4. Update relevant in-repository `.agents/skills/*` guidance.
5. Keep these surfaces aligned in the same change unless there is a documented reason not to.

`examples/readme` is the canonical source of truth for usage examples.

## Workspace Map

### Main User-Facing Entry Points

- `crates/es-fluent`
  Audience: **User-facing**
  Docs: [Architecture](crates/es-fluent/docs/ARCHITECTURE.md)
  Role: ecosystem facade, default entry point, and home of the registry types (`FtlTypeInfo`, `FtlVariant`, `RegisteredFtlType`).

- `crates/es-fluent-cli`
  Audience: **User-facing**
  Docs: [Architecture](crates/es-fluent-cli/docs/ARCHITECTURE.md)
  Role: primary CLI (`cargo es-fluent`) for generating, checking, cleaning, syncing, formatting, and inspecting FTL files.

- `crates/es-fluent-manager-embedded`
  Audience: **User-facing**
  Docs: [Architecture](crates/es-fluent-manager-embedded/docs/ARCHITECTURE.md)
  Role: zero-setup backend for embedding FTL files in the binary.

- `crates/es-fluent-manager-dioxus`
  Audience: **User-facing**
  Docs: [Architecture](crates/es-fluent-manager-dioxus/docs/ARCHITECTURE.md)
  Role: Dioxus integration for provider/hook client localization and request-scoped SSR.

- `crates/es-fluent-manager-bevy`
  Audience: **User-facing**
  Docs: [Architecture](crates/es-fluent-manager-bevy/docs/ARCHITECTURE.md)
  Role: Bevy integration for ECS and assets.

- `crates/es-fluent-lang`
  Audience: **User-facing**
  Docs: [Architecture](crates/es-fluent-lang/docs/ARCHITECTURE.md)
  Role: runtime language identification and localized language names for UI language pickers.

### Public Integration Crates

- `crates/es-fluent-derive`
  Audience: **Public integration**
  Docs: [Architecture](crates/es-fluent-derive/docs/ARCHITECTURE.md)
  Role: proc-macro crate for `#[derive(EsFluent)]`. Most users should depend on `es-fluent` instead of this crate directly.

- `crates/es-fluent-lang-macro`
  Audience: **Public integration**
  Docs: [Architecture](crates/es-fluent-lang-macro/docs/ARCHITECTURE.md)
  Role: generates type-safe language enums from asset folders. Most users should access this through `es-fluent-lang`.

- `crates/es-fluent-build`
  Audience: **Public integration**
  Docs: [Architecture](crates/es-fluent-build/docs/ARCHITECTURE.md)
  Role: build-script helper crate for locale asset rebuild tracking.

- `crates/es-fluent-manager-core`
  Audience: **Public integration**
  Docs: [Architecture](crates/es-fluent-manager-core/docs/ARCHITECTURE.md)
  Role: abstract traits for localization backends (`I18nModule`, `Localizer`).

- `crates/es-fluent-manager-macros`
  Audience: **Public integration**
  Docs: [Architecture](crates/es-fluent-manager-macros/docs/ARCHITECTURE.md)
  Role: macros for asset discovery and module registration. Most users consume this indirectly through manager crates.

### Internal Crates

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

### Examples and Web Surfaces

- `examples/example-shared-lib`
  Shared example library used by multiple examples.

- `examples/bevy-example`
  Bevy integration example using `es-fluent-manager-bevy`.

- `examples/gpui-example`
  GPUI integration example using `es-fluent-manager-embedded`.

- `examples/readme`
  Canonical executable documentation examples. Keep this in sync with the root `README.md` and the book.

- `web`
  Audience: **User-facing**
  Role: Dioxus-rendered GitHub Pages site hosting WASM demos and the mdBook; also an example for `es-fluent-manager-dioxus`.

- `book`
  Audience: **User-facing**
  Role: mdBook for public workflows and public crate usage.

## Validation and Editing Rules

### Validation After Changes

- Validation is the default after code or workflow changes.
- Run the narrowest command that proves the edited behavior works for the
  affected crate, docs, example, or web surface.
- Prefer targeted crate, example, docs, or web checks before full-workspace validation.
- Use `just check`, `just test`, or a more specific `justfile` recipe when the change spans multiple surfaces.
- If validation cannot be run, state why and what remains unvalidated.
- Do not claim a change works unless it was validated, generated from a source of truth, or the remaining risk is explicitly documented.

### When Editing Docs

- Keep READMEs and the book user-facing.
- Move implementation detail into `docs/ARCHITECTURE.md`.
- Prefer examples over prose-only explanations.
- Sync `examples/readme`, relevant READMEs, and book pages in the same change.

### When Editing Rust Crates

- Use `cargo` for build, test, and run tasks.
- Keep dependency versions in the workspace root `Cargo.toml`.
- Use `workspace = true` in member crates.
- Let each crate choose its own dependency features in its own `Cargo.toml`.
- Use `path` dependencies only in the root `Cargo.toml` and in examples.
- Non-example crates should reference workspace crates with `workspace = true`, not explicit paths.

### When Writing Tests

- Prefer [insta](https://insta.rs/) for snapshot tests when it fits better than assertion-heavy unit tests.
- Prefer raw multiline strings, or `quote! { ... }` in macro contexts, over escaped single-line literals for embedded Rust code.

### When Editing JavaScript/Typescript

- Use [bun](https://bun.com/) for dependency management and running scripts.
