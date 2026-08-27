# es-fluent-cli

[![Docs](https://docs.rs/es-fluent-cli/badge.svg)](https://docs.rs/es-fluent-cli/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-cli.svg)](https://crates.io/crates/es-fluent-cli)

The Cargo subcommand for generating, validating, synchronizing, formatting,
inspecting, and cleaning `es-fluent` FTL resources.

## Install

~~~sh
cargo install es-fluent-cli --locked
~~~

Run it as `cargo es-fluent <COMMAND>` or invoke the installed
`cargo-es-fluent` binary directly.

## Common workflow

~~~sh
cargo es-fluent doctor
cargo es-fluent generate
cargo es-fluent status --all-locales
cargo es-fluent check --all-locales
~~~

| Command | Purpose |
| --- | --- |
| `generate` | Update fallback FTL from derives. |
| `watch` | Regenerate when Rust or configuration inputs change. |
| `check` | Validate locale setup and Rust/FTL alignment. |
| `status` | Preview pending localization work without editing files. |
| `doctor` | Diagnose configuration, build wiring, managers, and fallback catalog readiness. |
| `fmt` | Format selected FTL resources. |
| `sync` | Seed missing keys into existing locales. |
| `add-locale` | Create and seed locale directories. |
| `clean` | Remove stale generated entries or orphaned files. |
| `tree` | Inspect resource files, entries, and variables. |

Run `cargo es-fluent doctor` for a read-only setup report. It follows Cargo's
selected library and custom-build targets and parses their local module graphs.
Add `--output json` for machine-readable configuration, build-script, manager,
policy, and catalog checks. Warnings identify constructs that static inspection
cannot prove and require manual verification.

During `watch`, transient Cargo metadata errors are reported while the previous
build-source graph and watches remain active. Saving a corrected manifest
retries metadata discovery without restarting the session.

Inspect fallback resources or every locale:

~~~sh
cargo es-fluent tree
cargo es-fluent tree --all-locales
~~~

Commands that inspect derives collect inventory from Cargo library targets.
Move binary-only localizable types into `src/lib.rs` or another
library module. A hidden inventory mode lets `generate` discover new derived
keys before strict fallback coverage is complete without changing the package's
configured runtime policy.

Use `--path` for a crate or workspace and `--package` for
one configured package. Preview destructive cleanup and aggressive generation
with `--dry-run`.

Machine-readable output is available for `check`, `doctor`, `fmt`,
`sync`, `tree`, and `status`:

~~~sh
cargo es-fluent check --all-locales --output json
~~~

## GitHub Actions

Run the repository-owned action from a workflow step:

~~~yaml
- uses: stayhydated/es-fluent/crates/es-fluent-cli@master
  with:
    path: .
    all_locales: true
    no_fallback_copy_check: false
~~~

Set `no_fallback_copy_check` to `true` only when all-locale validation should
allow translations that match the fallback text.

See the [CLI reference](https://stayhydated.github.io/es-fluent/book/cli.html)
for configuration, command behavior, workspace selection, CI, and recovery
guidance.
