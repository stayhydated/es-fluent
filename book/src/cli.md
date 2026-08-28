# CLI reference

`es-fluent-cli` maintains the FTL resources for a crate or Cargo
workspace. It can generate fallback entries, validate translations, synchronize
locales, format files, inspect resource trees, and clean stale output.

Commands that inspect derived messages collect inventory from library targets.
Keep localizable types reachable from `src/lib.rs` or another library
module.

## Install the CLI

~~~sh
cargo install es-fluent-cli --locked
~~~

Examples use Cargo's subcommand form:

~~~sh
cargo es-fluent --help
cargo es-fluent generate --help
~~~

The installed `cargo-es-fluent` binary accepts the same commands
directly.

Before running commands, create [`i18n.toml`](configuration.md) and
the configured fallback locale directory.

## Command overview

| Command | Use it to |
| --- | --- |
| `generate` | Add or update fallback FTL entries from Rust derives. |
| `watch` | Regenerate while Rust and configuration inputs change. |
| `check` | Validate configuration, keys, variables, locales, and orphaned files. |
| `status` | Preview pending generation, cleanup, formatting, sync, and validation work. |
| `doctor` | Diagnose configuration, build wiring, managers, and fallback catalog readiness. |
| `fmt` | Format selected FTL resources. |
| `sync` | Copy missing fallback keys into existing target locales. |
| `add-locale` | Create target locale directories and seed their FTL files. |
| `clean` | Remove entries or files not represented by current derive inventory. |
| `tree` | Inspect discovered resources, entries, attributes, and variables. |

Run `cargo es-fluent <COMMAND> --help` for the complete option set.

## Select crates and workspaces

Commands use the current directory by default.

- `--path <PATH>` or `-P <PATH>` selects a crate,
  workspace, manifest, or path inside a member.
- A workspace-root path selects every configured package.
- A member path selects that member.
- `--package <NAME>` or `-p <NAME>` selects one configured
  package from the workspace.
- `check --ignore <NAME>` excludes configured packages. Do not combine
  `--ignore` with `--package`.

Package-filtered commands avoid unrelated package configuration and compilation.
A filter that selects no configured package exits non-zero.

## Diagnose setup

Run the read-only setup doctor before generation or when compiler diagnostics
report missing catalog wiring:

~~~sh
cargo es-fluent doctor
cargo es-fluent doctor --output json
~~~

`doctor` checks `i18n.toml`, fallback locale and FTL catalog inputs, Cargo's
selected library and custom-build targets, the `es-fluent-build` build
dependency and `track_i18n_assets()` call, concrete manager declarations and
features, `define_i18n_module!()` registration, and the package-local strict or
`fallback-str` policy. It parses the local module graphs rooted at the selected
targets, so comments, strings, unreferenced files, and an unused root `build.rs`
do not count as integration evidence. Errors produce a non-zero exit code.
Warnings identify cases where static inspection cannot prove the integration
and request manual verification.

## Generate fallback resources

~~~sh
cargo es-fluent generate
~~~

Conservative mode is the default: it adds derived entries, updates their
declared variables, and preserves existing translations and manual-only
entries. Use aggressive mode only when generated resources should be rebuilt
from current derive inventory:

~~~sh
cargo es-fluent generate --mode aggressive --dry-run
~~~

`--dry-run` previews changes without writing them.
`--force-run` refreshes cached derive inventory. Generation may
compile selected library targets. A hidden inventory mode defers strict
missing-key coverage only for that temporary build, so new keys can be collected
without changing the package's configured runtime policy. Catalog parsing is
deferred to the requested operation so its normal validation and transaction
diagnostics remain authoritative.

`watch` runs the same generation flow when Rust, manifest, build
script, configuration, or workspace lockfile inputs change. Applicable
`.cargo/config.toml` and `.cargo/config` files in the workspace hierarchy and
Cargo home, their recursive includes, and configured lockfile paths invalidate
both watch fingerprints and cached derive inventory. Press `q` or `Ctrl-C` to
stop after active work and any already queued rerun finish. Transient Cargo
metadata errors keep the previous build-source graph and watches active; saving
a corrected manifest retries metadata discovery.

## Validate before committing

Check all locale directories:

~~~sh
cargo es-fluent check --all-locales
cargo es-fluent status --all-locales
~~~

`check` exits non-zero for setup or validation issues. It verifies
derived keys and arguments, canonical locale names, package-local ID
uniqueness, non-fallback coverage, and orphaned non-fallback files.

When translated text intentionally matches the fallback value, place this
marker before the message:

~~~ftl
# es-fluent: same-as-fallback
product-name = es-fluent
~~~

Alternatively, set `check_fallback_copies = false` for that package.

`status` does not edit project or locale files. It reports whether
generation, cleanup, formatting, synchronization, or validation needs
attention, making it the useful pre-commit summary.

## Format and manage locales

Format fallback resources or every discovered locale:

~~~sh
cargo es-fluent fmt
cargo es-fluent fmt --all-locales
~~~

Seed a new locale:

~~~sh
cargo es-fluent add-locale fr-FR
~~~

Synchronize existing locale directories:

~~~sh
cargo es-fluent sync --all-locales
cargo es-fluent sync --locale fr-FR --dry-run
~~~

Use `sync --create --locale <LANG>` when scripts need explicit locale
creation, including JSON output. `--all-locales` processes existing
locale directories and cannot be combined with `--create`.

Commands that write locale files plan the selected workspace change before
committing it. A failed write restores earlier changes from that command.

## Clean stale resources

`clean` treats current derive inventory as the source of truth for
selected package and domain resources. It can remove manual-only entries and
empty package-owned files, so preview it first:

~~~sh
cargo es-fluent clean --dry-run
cargo es-fluent clean --all-locales --dry-run
~~~

Add `--orphaned` to find non-fallback FTL files that have no matching
fallback resource:

~~~sh
cargo es-fluent clean --orphaned --dry-run
~~~

Remove `--dry-run` only after reviewing the planned deletions.

## Inspect resources

~~~sh
cargo es-fluent tree
cargo es-fluent tree --all-locales
cargo es-fluent tree --output json
~~~

Text output can link message rows to Rust or FTL source locations. Use
`--link-mode ftl` for file-only inspection that does not compile a
library target. JSON output is file-oriented and does not accept
`--link-mode`.

## Structured output

`check`, `fmt`, `sync`, `tree`, and
`status` support `--output json`. After successful argument
parsing, JSON mode writes the report to stdout. Use both the process exit status
and documented report fields when automation distinguishes errors, warnings, or
pending dry-run work.

## GitHub Actions

The repository publishes an action that runs `cargo es-fluent check`:

~~~yaml
name: es-fluent
on: [pull_request]

jobs:
  localization:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - name: Check Fluent resources
        uses: stayhydated/es-fluent/crates/es-fluent-cli@<TAG_OR_SHA>
        with:
          path: .
          all_locales: true
          no_fallback_copy_check: false
~~~

Pin the action to a release tag or commit SHA for reproducible builds.
Set `no_fallback_copy_check` to `true` only when all-locale validation should
allow translations that match the fallback text.
