# AGENTS.md

This is the working guide for contributors and coding agents in the `es-fluent`
workspace.

Use it to decide:

1. whether a crate or surface is user-facing, public integration, or internal,
2. where documentation and durable source-of-truth details belong,
3. which related code docs, user docs, examples, generated outputs, and skills
   must change together,
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
2. **Keep source-of-truth details close to the thing they describe.** Use code,
   tests, Rust docs, module docs, local comments, examples, READMEs, and the
   book instead of standalone architecture documents.
3. **Assert code/docs sync before handoff.** For any API, CLI, generated-output,
   or workflow change, check the implementation, tests, Rust docs/module docs,
   user-facing docs, examples, and skill guidance that describe the same behavior.
4. **Sync public workflow changes.** If behavior, commands, generated output,
   or recommended usage changes, update the relevant example, README, book page,
   and `skills/use-es-fluent` guidance in the same change when applicable.
5. **Validate narrowly.** Run the smallest command that proves the edited
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

### Source-of-Truth Documentation

Use code and its associated documentation as the source of truth:

- API contracts belong in Rust docs and public examples.
- Public crate READMEs are often included as crate docs with
  `#![doc = include_str!("../README.md")]`; keep README content, Rust docs, and
  examples aligned when public APIs change.
- Internal invariants belong in tests, snapshots, module docs, or focused code
  comments.
- Maintenance workflow guidance belongs in this file or the relevant tool docs.
- Public workflows belong in READMEs, `examples/readme`, the book, and
  `skills/use-es-fluent` when applicable.

Do not add standalone `docs/ARCHITECTURE.md` files. If a deleted architecture
document contained important durable knowledge, move that knowledge into the
nearest code, test, README, or book page that owns the behavior.

### Skill Guidance

`skills/use-es-fluent` is public application-developer guidance, not repo-local
maintenance guidance. Keep maintainer-only details in this guide or near the
code they affect.

Update it when user-facing workflows, CLI behavior, generated output,
integration patterns, or recommended usage change.

## Synchronization Rules

When a substantive change modifies a public workflow, public feature, or
user-visible API shape:

1. Update the implementation and the Rust docs/module docs that own the API
   contract.
2. Update the executable example in `examples/readme` when relevant.
3. Update the affected user-facing `README.md` files.
4. Update the matching `book/src/*.md` pages.
5. Update `skills/use-es-fluent` when public usage guidance changes.
6. Keep these surfaces aligned in the same change unless there is a documented reason not to.

`examples/readme` is the canonical source of truth for usage examples.

For CLI behavior, keep `crates/es-fluent-cli/README.md`, `book/src/cli.md`,
`skills/use-es-fluent/references/cli-workflow.md`, and the relevant root README
sections in sync. `crates/es-fluent-cli/tests/main_smoke.rs` contains tests that
assert parts of this public documentation contract.

Generated outputs are not independent source-of-truth files. Update their
sources or generator first:

- `.ftl` files and `.es-fluent/metadata` come from es-fluent CLI discovery and
  generation.
- `book/book`, `web/public/book`, `web/public/llms*`, demo bundles under
  `web/public`, and `web/dist` come from `xtask`/web build commands.

## Workspace Map

### Main User-Facing Entry Points

- `crates/es-fluent`
  Audience: **User-facing**
  Role: ecosystem facade, default entry point, and home of the registry types (`FtlTypeInfo`, `FtlVariant`, `RegisteredFtlType`).

- `crates/es-fluent-cli`
  Audience: **User-facing**
  Role: primary CLI (`cargo es-fluent`) for generating, checking, cleaning, syncing, formatting, and inspecting FTL files.

- `crates/es-fluent-manager-embedded`
  Audience: **User-facing**
  Role: zero-setup backend for embedding FTL files in the binary.

- `crates/es-fluent-manager-dioxus`
  Audience: **User-facing**
  Role: Dioxus integration for provider/hook client localization and request-scoped SSR.

- `crates/es-fluent-manager-bevy`
  Audience: **User-facing**
  Role: Bevy integration for ECS and assets.

- `crates/es-fluent-lang`
  Audience: **User-facing**
  Role: runtime language identification and localized language names for UI language pickers.

### Public Integration Crates

- `crates/es-fluent-derive`
  Audience: **Public integration**
  Role: proc-macro crate for `#[derive(EsFluent)]`. Most users should depend on `es-fluent` instead of this crate directly.

- `crates/es-fluent-lang-macro`
  Audience: **Public integration**
  Role: generates type-safe language enums from asset folders. Most users should access this through `es-fluent-lang`.

- `crates/es-fluent-build`
  Audience: **Public integration**
  Role: build-script helper crate for locale asset rebuild tracking.

- `crates/es-fluent-manager-core`
  Audience: **Public integration**
  Role: abstract traits for localization backends (`I18nModule`, `Localizer`).

- `crates/es-fluent-manager-macros`
  Audience: **Public integration**
  Role: macros for asset discovery and module registration. Most users consume this indirectly through manager crates.

### Internal Crates

- `crates/es-fluent-shared`
  Audience: **Internal**
  Role: shared runtime-safe types, naming helpers, namespace rules, path utilities, and common errors.

- `crates/es-fluent-derive-core`
  Audience: **Internal**
  Role: shared build-time derive logic, including option parsing and validation.

- `crates/es-fluent-toml`
  Audience: **Internal**
  Role: `i18n.toml` parsing and path resolution shared by macros and CLI tooling.

- `crates/es-fluent-cli-helpers`
  Audience: **Internal**
  Role: runtime logic executed inside the temporary runner crate.

- `crates/es-fluent-runner`
  Audience: **Internal**
  Role: runner protocol types and `.es-fluent/metadata` path helpers.

- `crates/es-fluent-generate`
  Audience: **Internal**
  Role: FTL AST manipulation, diffing, formatting, and merge behavior.

- `xtask`
  Audience: **Internal**
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
  Role: Dioxus-rendered GitHub Pages site hosting WASM demos, `llms.txt`, and
  the mdBook; also an example for `es-fluent-manager-dioxus`.

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
- Use `cargo es-fluent-local check --path . --all` or the relevant
  `cargo es-fluent-local` subcommand when proving generated FTL output or
  locale metadata is current in this repository.
- Use `just test-docs` for Rust documentation builds, `cargo xtask build book`
  for mdBook output, `cargo xtask build llms-txt` for `llms.txt` output, and
  `cargo xtask build web`/`just web-build` for published web artifacts when
  those surfaces are affected.
- If validation cannot be run, state why and what remains unvalidated.
- Do not claim a change works unless it was validated, generated from a source of truth, or the remaining risk is explicitly documented.

### When Editing Docs

- Keep READMEs and the book user-facing.
- Keep implementation detail near the code, tests, or module docs that own it.
- Do not add standalone `docs/ARCHITECTURE.md` files.
- Prefer examples over prose-only explanations.
- Sync `examples/readme`, relevant READMEs, book pages, and
  `skills/use-es-fluent` when public usage guidance changes.

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
