# Working in es-fluent

`es-fluent` provides typed Fluent messages, runtime managers, and FTL tooling.
Start with the owning crate below and `just --list` for repository commands.

## Where to make changes

| Surface | Owner and audience |
| --- | --- |
| Application API | `crates/es-fluent`: public traits and derive re-exports. |
| CLI and GitHub Action | `crates/es-fluent-cli`: commands, workspace selection, watching, and the action wrapper. |
| Runtime integrations | `crates/es-fluent-manager-embedded`, `es-fluent-manager-dioxus`, and `es-fluent-manager-bevy`: application managers. |
| Language picker | `crates/es-fluent-lang` and `es-fluent-lang-macro`: locale enums and language labels. |
| Derive implementation | `crates/es-fluent-derive` and `es-fluent-derive-core`: macro expansion and validation. |
| Shared runtime contracts | `crates/es-fluent-manager-core` and `es-fluent-manager-macros`: custom integrations and module registration. |
| Configuration and generation | `crates/es-fluent-toml`, `es-fluent-build`, and `es-fluent-generate`: layouts, fallback catalogs, and FTL maintenance. |
| Internal protocols | `crates/es-fluent-shared`, `es-fluent-runner`, and `es-fluent-cli-helpers`: metadata, runner planning, and filesystem transactions. |
| User guidance | Root and crate READMEs, `book/src`, and `skills/use-es-fluent`. |
| Executable examples | `examples/readme`, `examples/bevy-example`, `examples/gpui-example`, and `examples/example-shared-lib`. |
| Site and build tooling | `web` contains the Dioxus site and demo; `xtask` builds the site, book, LLM exports, and hosted demos. |

Crate paths in a row share the `crates/` prefix. Application code normally uses
the facade and a concrete manager; supporting crates serve narrower integration
or implementation work.

## Keep related surfaces aligned

- Public API and manager changes belong with the matching crate README, Rust
  docs, book pages, executable examples, and skill reference. Several crate
  roots include their README as Rust documentation.
- Keep README badges for CI, Codecov, the book, and the published crate. Keep
  setup procedures and chapter navigation in the book; READMEs explain purpose,
  public behavior, and representative usage.
- CLI changes must agree with `crates/es-fluent-cli/README.md`,
  `book/src/cli.md`, and
  `skills/use-es-fluent/references/cli-workflow.md`.
  `crates/es-fluent-cli/tests/main_smoke/help.rs` checks action usage and common
  documentation contracts.
- Update affected `.ftl` files and inventory expectations when localizable
  types or `i18n.toml` change. Use the repository's `cargo es-fluent-local`
  alias to generate or validate locale resources with the local CLI.
- Keep derive diagnostics, UI `.stderr` files, and `insta` snapshots aligned
  with expansion and generated-output changes. Read the owning tests before
  changing expected output.
- Use `tests/fixtures/multi-crate` for changes involving package identity,
  custom library targets, dependency aliases, manager resource plans, or
  cross-package CLI and runtime behavior.
- Edit book, web, and demo sources before rebuilding their outputs through
  `xtask`. Keep ownership and validation guidance here aligned when those
  workflows change.

## Cross-crate contracts

### Package ownership and fallback messages

- Package plus domain owns generated FTL. An omitted derive domain uses the
  Cargo package name; an explicit domain must be declared in that package's
  `i18n.toml`. Namespaces split resources within that ownership boundary.
- Validate selected packages independently. Reusing a domain or ID in another
  package is valid. Within one package-local domain, messages and terms share
  one runtime ID namespace; duplicate messages, duplicate terms, and
  message/term collisions are errors.
- Keep orphan cleanup scoped to the selected package and conservative about
  unselected fallback-relative paths. Raw filenames alone do not establish
  global ownership.
- `es-fluent-build` writes the fallback catalog consumed by derives. `strict`
  is the default missing-message policy; `fallback-str` puts snake_case source
  fallbacks on that package's generated keys. Fallible lookup returns `None`.
  Strict and fallback-string packages can coexist in one build.
- Policy changes must agree across catalog parsing, source-spanned derive
  diagnostics, `doctor`, embedded/Dioxus/Bevy lookup, and compile/runtime tests.
  The CLI's temporary inventory environment must preserve application policy.

### CLI planning, writes, and watching

- `generate`, `clean`, `fmt`, `sync`, and `add-locale` plan the complete selected
  change before writing. Runner-backed commands return plans; the CLI host
  commits only after every selected plan succeeds.
- Preserve before-state verification and rollback across selected packages,
  locale directories, and files. Keep runner protocol types, transaction tests,
  JSON applied/changed counts, and `status` previews consistent with the actual
  committed result. Start with `crates/es-fluent-runner/src/transaction.rs` and
  `crates/es-fluent-cli/src/commands/common/generation.rs`.
- Keep `sync` results resource-identifiable. JSON `results[].path` is
  workspace-relative; `null` identifies directory-only creation for an empty
  fallback locale.
- Cargo metadata owns library and custom-build target paths. `doctor` follows
  their parsed local module graphs; unsupported static-analysis constructs are
  warnings requiring manual verification.
- Cache and watch inputs include the selected custom-build target and reachable
  local modules. Applicable Cargo configuration files in the workspace
  hierarchy and Cargo home, recursive includes, and configured lockfile paths
  share the same cache invalidation and watching contract.
- Watch shutdown through `q` or `Ctrl-C` waits for active generation and any
  rerun already queued by a mid-generation input change.

## Validate the changed surface

Use the narrowest relevant check first. The `justfile` and CI workflow contain
broader checks for changes spanning several surfaces.

| Change | Validation |
| --- | --- |
| One crate | `cargo test -p <package> --locked` with the affected test or feature selection. |
| Public CLI documentation | `cargo test -p es-fluent-cli --test main_smoke public_ --locked`. |
| Rust documentation | `just test-docs` builds workspace docs and opens the result. |
| Dioxus manager features | `just test-dioxus-manager-feature-matrix`. |
| Locale setup | `just doctor` for read-only configuration and wiring diagnostics. |
| FTL resources or inventory | `cargo es-fluent-local check --path . --all-locales`. |
| Markdown | `rumdl check` with the edited paths. |
| Book | `MDBOOK_BUILD__CREATE_MISSING=false cargo xtask build book`. |
| LLM exports | `cargo xtask build llms-txt`. |
| Hosted demos | `cargo xtask build bevy-demo` or `cargo xtask build gpui-demo`. |
| Dioxus integration | `cargo test -p web --lib` or `cargo check -p web`. |
| Complete site | `just web-build`. |

Use `just check`, `just clippy`, or `just test` when a change crosses workspace
boundaries. Report commands that succeeded separately from failed attempts and
checks that were only reviewed.
