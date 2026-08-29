# AGENTS.md

This is the working guide for contributors and coding agents in the `es-fluent`
workspace.

Use it to decide:

- which crate or surface owns a change,
- whether the surface is user-facing, public integration, generated, validation,
  or internal,
- which docs, examples, generated outputs, fixtures, skill references, and tests
  must stay synchronized,
- which repository command is the narrowest useful validation.

Start with `crates/es-fluent` for the public application API,
`crates/es-fluent-cli` for CLI behavior, and `just --list` for the repository
command index.

## Project Summary

`es-fluent` is a Rust localization workspace built on Project Fluent. It
provides typed derive macros, runtime managers, CLI tooling, generated FTL file
maintenance, examples, an mdBook, and a Dioxus-rendered web site.

## Quick Decision Flow

1. Find the edited surface in the workspace map and use its audience label to
   decide which public docs or validation surfaces are part of the same change.
2. Keep implementation details close to the code, Rust docs, tests, fixtures,
   snapshots, examples, generator inputs, or source docs that prove the behavior.
3. For public API, CLI, generated-output, or workflow changes, update the owning
   implementation, tests, Rust docs, user-facing docs, examples, and
   `skills/use-es-fluent` guidance that describe the same behavior.
4. For generated or ignored outputs, change the source, generator, example,
   book source, or web source first; regenerate with the evidenced command when
   the output itself matters.
5. Validate with the narrowest command that proves the edited behavior or docs
   surface.

## Audience Labels

- **User-facing**: default application crates, CLI behavior, examples, public
  READMEs, the mdBook, and the published web site.
- **Public integration**: crates or macros intended for custom integrations or
  deeper extension work.
- **Generated/source-of-truth**: generators, ignored generated outputs, locale
  metadata, and source files that produce generated artifacts.
- **Validation**: tests, fixtures, snapshots, UI stderr files, and examples that
  encode expected behavior.
- **Internal**: workspace plumbing, implementation details, and maintenance
  tooling.

## Workspace Map

### Main User-Facing Surfaces

- `crates/es-fluent`: facade crate, derive re-exports, public runtime traits,
  and default application entry point.
- `crates/es-fluent-cli`: `cargo es-fluent` binary, public GitHub Action
  wrapper, and CLI docs.
- `crates/es-fluent-manager-embedded`, `crates/es-fluent-manager-dioxus`,
  `crates/es-fluent-manager-bevy`: runtime managers for embedded/general Rust,
  Dioxus client/SSR, and Bevy.
- `crates/es-fluent-lang`: typed language enum and localized language labels.
- `crates/es-fluent-build`: build-script helper for tracking locale asset
  rebuilds.

### Public Integration Surfaces

- `crates/es-fluent-derive`: proc-macro crate behind the facade derives.
- `crates/es-fluent-lang-macro`: proc macro behind `es-fluent-lang`.
- `crates/es-fluent-manager-core`: shared runtime contracts and manager
  abstractions.
- `crates/es-fluent-manager-macros`: manager module registration and Bevy text
  macros.

### Internal Implementation and Tooling

- `crates/es-fluent-shared`, `crates/es-fluent-derive-core`,
  `crates/es-fluent-toml`, `crates/es-fluent-generate`,
  `crates/es-fluent-cli-helpers`, `crates/es-fluent-runner`: shared metadata,
  derive validation, config parsing, FTL generation, runner helpers, and runner
  protocol types.
- `xtask`: repository maintenance commands for generated book, `llms.txt`, demo
  bundles, web builds, and release planning.

### Docs, Examples, and Web

- `README.md` and crate `README.md` files are user-facing. Many crate READMEs
  are included as crate docs with `#![doc = include_str!("../README.md")]`.
- `book/src`: mdBook source for public workflows. `book/book` is generated.
- `examples/readme`: canonical executable README examples; keep it aligned with
  root README and relevant book pages.
- `examples/bevy-example`, `examples/gpui-example`,
  `examples/example-shared-lib`: integration examples and shared example code.
  Their Trunk page inputs are staged by the owning `xtask` build commands
  through `stayhydated-xtask`.
- `examples/typescript-example`, `examples/solid-example`,
  `examples/react-example`, `examples/expo-example`: self-contained
  Rust-derived contracts, generated TypeScript exports, and runnable target
  applications. All four build into hosted browser demos; Expo also targets
  iOS and Android through its native runtime.
- `web`: Dioxus-rendered GitHub Pages site and Dioxus integration example.
- `packages/es-fluent-ts`, `packages/es-fluent-solid`,
  `packages/es-fluent-react`: framework-neutral TypeScript runtime, Solid
  2/SolidStart integration, and React 19 web/SSR integration for exported
  Rust-owned localization contracts.
- `packages/es-fluent-expo`: universal Expo facade, TypeScript web adapter,
  React facade re-exports, iOS/Android module, Swift and Kotlin wrappers, native
  build scripts, and checked-in UniFFI bindings. `packages/es-fluent-expo/rust`
  owns its native Rust Project Fluent runtime.

### Generated and Validation Surfaces

- `.es-fluent/`: ignored local runner workspace and metadata generated by the
  local CLI alias.
- `packages/es-fluent-expo/ios/generated` and
  `packages/es-fluent-expo/android/src/main/java/expo/modules/esfluent/uniffi`:
  checked-in bindings generated by `npm run generate:bindings --workspace
  @es-fluent/expo`.
- `packages/es-fluent-expo/.native-build`, Android `jniLibs`, and the iOS
  XCFramework are ignored native build outputs.
- `web/public/book`, `web/public/llms*`, `web/public/llms/`,
  `web/public/bevy-demo`, `web/public/gpui-demo`,
  `web/public/typescript-demo`, `web/public/solid-demo`,
  `web/public/react-demo`, `web/public/expo-demo`, and `web/dist`: ignored
  outputs from `xtask` and web build commands.
- `crates/*/tests`, `crates/*/tests/fixtures`,
  `crates/*/tests/snapshots`, `crates/*/src/snapshots`,
  `crates/*/tests/ui/*.stderr`, and `*.snap` files: validation contracts for
  CLI behavior, derive diagnostics, generated FTL output, manager macros, and
  runtime behavior.

## Synchronization Rules

- When public API or manager behavior changes, update the owning crate README,
  Rust docs/module docs, relevant examples, root README, and matching
  `book/src/*.md` pages.
- Treat configured fallback-message coverage as a package-local compile-time
  contract. The `es-fluent-build` helper writes the fallback catalog consumed by
  derives; `missing_message_policy = "strict"` is the default, while
  `"fallback-str"` puts snake_case source-name fallback metadata on that
  package's generated keys across embedded, Dioxus client/SSR, and Bevy.
  Fallible lookup remains `None`, and strict and fallback-string packages may
  coexist in one workspace build. The CLI uses a hidden inventory environment
  without changing application policy. Keep source-spanned diagnostics, shared
  catalog parsing, `doctor` checks, docs, skills, and compile/runtime tests
  synchronized.
- Treat Cargo metadata as authoritative for library and custom-build target
  paths. `doctor` verifies calls and manager registration through parsed local
  module graphs; cache and watch inputs follow the selected custom-build target
  and its reachable local modules. Applicable Cargo configuration files from the
  workspace hierarchy and Cargo home, their recursive includes, and configured
  lockfile paths share one watcher and runner-cache invalidation contract.
  Unsupported static-analysis constructs are warnings, not passes.
- When CLI behavior changes, keep `crates/es-fluent-cli/README.md`,
  `book/src/cli.md`, `skills/use-es-fluent/references/cli-workflow.md`, and the
  relevant root README sections aligned. `crates/es-fluent-cli/tests/main_smoke.rs`
  asserts parts of this public documentation contract.
- When public usage guidance changes, update `skills/use-es-fluent` or the
  relevant reference file in the same change as docs and examples.
- When localizable Rust types, `i18n.toml`, or locale assets change, keep the
  affected `.ftl` assets, CLI metadata expectations, examples, and docs aligned.
  Use the local `cargo es-fluent-local` alias when generated FTL or metadata is
  the affected surface.
- Treat runner-backed `generate` and `clean` as filesystem transactions across
  all selected packages and locale files. Runner processes plan mutations and
  the CLI host commits only after every selected plan succeeds; keep
  before-state verification, rollback tests, runner protocol metadata, CLI
  documentation, and status cleanup previews synchronized.
- Treat `export typescript` as a filesystem transaction across the selected
  packages and output directory. Preserve package-plus-domain ownership,
  deterministic revisions, workspace-relative source metadata, state-owned
  stale cleanup, CLI docs, TypeScript runtime contracts, and Rust/TypeScript
  tests together.
- Keep the TypeScript core, Solid 2, React, and universal Expo facades on the
  same `EsFluentRuntime`, request, descriptor, snapshot, and revision contracts.
  Expo web selects the TypeScript runtime while iOS and Android use Rust through
  UniFFI. When its UniFFI interface changes, regenerate both Swift and Kotlin
  bindings and keep platform wrappers, build scripts, package docs, book
  guidance, and lifecycle tests synchronized.
- Keep each TypeScript target example's Rust derives, FTL, checked-in
  `generated/` export, application code, README, and Demos route synchronized.
  `cargo xtask build typescript-demos` owns the four browser bundles staged
  under `web/public`. The TypeScript demos route presents core, Solid, React,
  and Expo in one framework-labelled list; Dioxus, Bevy, and GPUI retain
  dedicated routes. The Expo entry uses the same frame as React and identifies
  `@es-fluent/expo` as its runtime package.
- Treat `fmt` as a filesystem transaction across every selected package and
  locale file. Plan all formatting before writing; keep rollback tests, JSON
  changed counts, CLI documentation, and status formatting previews
  synchronized.
- Treat `sync` and `add-locale` as filesystem transactions across every selected
  package, locale directory, and FTL file. Plan all mutations before writing,
  verify before-state at commit, and keep directory/file rollback tests, CLI
  documentation, and JSON applied counts/results synchronized.
- Keep `sync` text and JSON results resource-identifiable. JSON `results[].path`
  is workspace-relative and is `null` only for directory-only locale creation
  with an empty fallback locale.
- Keep fallback message and term keys synchronized. Messages and terms share one
  runtime Fluent ID namespace within each package-local domain, so `check`
  reports duplicate messages, duplicate terms, and message/term ID collisions.
- Treat `watch` shutdown as graceful: `q` and `Ctrl-C` wait for active
  generation and any rerun already queued by a mid-generation input change.
- Treat an omitted derive domain as generated FTL in the package-name resource.
  Treat an explicit `#[fluent(domain = "...")]` as generated FTL in an
  additional package-local resource declared by `domains` in that package's
  `i18n.toml`; domains do not form cross-crate references.
- Treat package plus domain as the generated-FTL ownership boundary.
  Full-workspace `cargo es-fluent-local check --path . --all-locales` and `status
  --all-locales` validate selected packages independently; the same domain filename or
  ID in another package is not a collision. Keep package-scoped orphan cleanup
  conservative about unselected fallback-relative paths, and do not infer
  ownership globally from raw filenames.
- Keep `tests/fixtures/multi-crate` aligned with package identity, custom
  library target, dependency alias, manager resource-plan, CLI, and runtime
  behavior changes.
- When book, web, demo, or `llms.txt` behavior changes, edit the source under
  `book/src`, `web/src`, `web/assets`, `examples`, or `xtask`, then rebuild the
  affected generated surface when needed.
- When a named command, boundary, generated-output flow, or public workflow in
  this guide changes, update `AGENTS.md` in the same change.

## Repository Standards

- Keep dependency versions and workspace path dependencies in root `Cargo.toml`.
  Member crates use `workspace = true`; examples may use path dependencies for
  local example crates.
- Keep user-facing docs example-first. Public crate README changes often need
  matching Rust docs because crate roots include README content.
- `insta` snapshots are a repository testing convention; use or update
  snapshots when they are the local validation surface for the changed behavior.

## Validation and Editing Rules

- Start with `just --list`; use the `justfile` rather than duplicating the full
  command inventory here. Use `just doctor` for read-only configuration,
  build-script, manager, registration, policy, and fallback-catalog diagnostics.
- For Rust code, prefer the narrowest package or surface check first. Use
  `just check`, `just clippy`, `just test`, or `just ci` when the change spans
  multiple workspace surfaces.
- For Dioxus manager feature behavior, run
  `just test-dioxus-manager-feature-matrix`.
- For generated FTL output or locale metadata, run
  `cargo es-fluent-local check --path . --all-locales`; use
  `cargo es-fluent-local fmt --all-locales` when formatting FTL files.
- For Rust documentation builds, run `just test-docs`.
- For the TypeScript runtime and Solid, React, and Expo facades, run
  `npm run test:typescript`.
- For the target applications, run `npm run typecheck:demos` and
  `cargo xtask build typescript-demos`; validate the Expo web target with
  `npm run build:web --workspace @es-fluent/example-expo` when the universal
  app surface changes.
- For the Expo Rust runtime, run `cargo test -p es-fluent-expo-native` and
  `cargo clippy -p es-fluent-expo-native --all-targets --all-features -- -D
  warnings`. Run the Android native build with its NDK targets and the iOS
  native build on macOS before claiming those platform artifacts compile.
- For release ordering or package metadata changes, run `just test-publish`; it
  uses `cargo xtask release plan`, matching the CI package job.
- CI also runs docs, release package-plan, cargo-machete, coverage, and Codecov
  publishing from `.github/workflows/ci.yml`.
- For mdBook, `llms.txt`, demos, and the published web surface, use the
  relevant `cargo xtask build book`, `cargo xtask build llms-txt`,
  `cargo xtask build bevy-demo`, `cargo xtask build gpui-demo`,
  `cargo xtask build typescript-demos`, `cargo xtask build web`, or the
  aggregate `just web-build`.
- For `web` integration changes covered by CI, use `cargo test -p web --lib`
  or `cargo check -p web`; the site is an unconditional browser/SSG package.
- If validation cannot be run, state why and what remains unvalidated. Do not
  claim a change works unless it was validated or the remaining risk is
  explicitly documented.
