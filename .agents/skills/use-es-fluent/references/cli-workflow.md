# CLI Workflow

Use this reference when scaffolding projects, generating FTL, checking translations, or maintaining generated locale files.

Examples use Cargo's subcommand form, `cargo es-fluent <COMMAND>`. The
installed binary also accepts direct invocation as
`cargo-es-fluent <COMMAND>`, so `cargo-es-fluent --help`,
`cargo-es-fluent generate --help`, and `cargo-es-fluent help generate` show the
same command surface as `cargo es-fluent --help`.

## Configuration

The standard `i18n.toml` lives next to the crate `Cargo.toml`:

```toml
fallback_language = "en"
assets_dir = "assets/locales"

# Optional: features needed to compile inventory for derives.
fluent_feature = ["my-feature"]

# Optional: restrict string namespace values.
namespaces = ["ui", "errors", "messages"]

# Optional: disable warnings when non-fallback messages copy fallback text.
check_fallback_copies = false
```

`assets_dir` is relative to the crate root, must stay inside the crate, and
must not use existing symlinked path components or locale targets.
Locale directory names and locale arguments should use canonical BCP-47 tags
such as `en`, `fr-FR`, and `zh-CN`. Deprecated aliases such as `iw` and `src`
are rejected; use canonical replacements such as `he` and `sc`.

For non-`init` commands, `--path` must be an existing workspace root, member
root, that member's `Cargo.toml`, or a path inside a member. Paths inside a
workspace member select that member by default; if the path includes a symlink
inside the member, selection follows the path as passed rather than the symlink
target.

## Scaffolding

For a new crate, prefer:

```sh
cargo es-fluent init --update-cargo-toml
```

Useful options:

- `--manager dioxus` or `--manager bevy`: use a framework-specific module scaffold.
- `--dioxus-runtime client`, `--dioxus-runtime ssr`, or `--dioxus-runtime "client, ssr"`: choose generated Dioxus features; requires `--manager dioxus` and `--update-cargo-toml`.
- `--build-rs`: create or update `build.rs` with locale asset rebuild tracking for manager macros.
- `--update-cargo-toml`: add missing `es-fluent`, manager, and `unic-langid` dependency entries; existing dependency entries are preserved and their version requirements are not replaced. The package `Cargo.toml` must be a real file, not a symlink.
- `--fallback-language <LANG>`: choose fallback locale.
- `--locales fr-FR,zh-CN`: create additional non-fallback locale directories; quote comma lists that include spaces, such as `--locales "fr-FR, zh-CN"`; values must not include the fallback locale.
- `--assets-dir <PATH>`: choose locale asset directory relative to the crate root; it must stay inside the crate, existing path components must not be symlinks, and existing locale directory targets reused by `init` must be real directories rather than symlinks. Passing `.` is accepted and creates locale directories such as `./en` directly under the crate root; all-locale scans then process canonical locale directories, ignore common project directories such as `src`, `target`, `bin`, and `lib`, and still report noncanonical locale-looking names such as `en-us`. `init`, `sync --create`, and `add-locale` reject newly created locale names that match those ignored project directories when `--assets-dir .` is used; for `init`, this includes the fallback locale directory. Explicit `--locale <LANG>` targets can still use an existing ignored-name directory; use a dedicated asset directory instead of `.` if you need `--all` to process such a locale.
- `--namespaces ui,errors`: write a namespace allowlist; quote comma lists that include spaces, such as `--namespaces "ui, errors"`.
- `--dry-run`: preview files and manifest updates without writing.
- `--force`: overwrite existing `i18n.toml` and i18n module scaffold targets; existing `build.rs` files are only patched when `init` can safely add the tracking call.

Comma-separated list options are trimmed, empty entries are rejected, and
duplicate additional-locale, namespace, and Dioxus runtime values are ignored
in generated output.

`init` creates or updates the crate's library target because inventory
collection reads library targets. By default that is `src/lib.rs`; if
`Cargo.toml` declares a custom `[lib].path`, `init` uses that path and writes
`i18n.rs` next to it. The chosen library path must remain inside the crate
root, resolve to a source file rather than a directory, must not use existing
symlinked path components, and must not itself be the generated `i18n.rs`
module path.
If the library already defines an inline `mod i18n { ... }`, move that module
to the generated `i18n.rs` path or remove it before running `init`.
Existing external `i18n` declarations must be public; change `mod i18n;` or
`pub(crate) mod i18n;` to `pub mod i18n;` before running `init`.
In a Cargo workspace, run `init` from the member crate or pass
`--path <member-crate>` or `--path <member-crate>/Cargo.toml`; virtual workspace
roots and manifests are rejected because they have no package library target to
update.
Plain `init` can scaffold missing es-fluent files in an existing Cargo package
directory, but it does not create `Cargo.toml`; the target must already have a
readable, parseable manifest with a `[package]` table.
When `--build-rs` updates an existing `build.rs`, existing build-script logic is
preserved if `init` can add `es_fluent_build::track_i18n_assets();` to
`fn main`; otherwise add the call manually. `--force` does not overwrite an
existing unpatchable `build.rs`.
Commands that need inventory collection fail when a configured crate has
`i18n.toml` but no Cargo library target. `clean --orphaned` without `--all` can
still run its file-only orphan scan for such crates, and skips runner
preparation when no selected crate has a library target. Combining `--all` with
`--orphaned` still requires library targets because the all-locale generated-key
clean must run before the orphan scan. File-only commands such as `fmt`, `sync`,
and `add-locale`, plus `tree --output json` and text `tree --link-mode ftl`,
can still inspect or edit discovered FTL files. Text `tree --link-mode rust`
also renders crates without library targets from discovered FTL files, but only
crates with library targets can contribute Rust source links. Commands that
print the shared discovery summary, such as `fmt` and `clean --orphaned`,
report the missing library target as a notice.
When a custom `[lib].path` exists, runner-backed commands require the existing
library target path components to stay inside the crate and avoid symlinks;
violations are reported before `.es-fluent` runner metadata or Cargo build
output is prepared.

## Routine Commands

After adding or changing derived localizable types:

```sh
cargo es-fluent generate
```

Generation updates fallback FTL, adds new messages, updates declared variables, and preserves existing translations in conservative mode.
Missing output directories are created when generation writes files, but
existing path components leading to `assets_dir` and the fallback locale must be
real directories, not symlinks; files such as `i18n` or `i18n/en` are reported
as setup errors before runner metadata or Cargo build output is prepared.
Existing fallback `.ftl` output paths, such as `i18n/en/my-crate.ftl`, must be
real files, not symlinks; directory-valued FTL paths are rejected before runner
metadata or Cargo build output is prepared.
For crates with a custom `[lib].path`, existing library target path components
must be real in-crate paths, not symlinks; symlinked or escaping library target
paths are reported before runner metadata or Cargo build output is prepared.
If a generated `i18n.rs` module exists, runner-backed generation also requires
that path to be a real file and the library target to declare it with
`pub mod i18n;`; symlinked or non-file module paths, inline `mod i18n { ... }`
definitions, non-public declarations, and missing declarations are reported
before runner metadata or Cargo build output is prepared.
With `--dry-run`, generation previews locale-file changes without editing FTL
files, but it may still prepare `.es-fluent` runner metadata and Cargo build
output while collecting Rust inventory. If `.es-fluent` already exists, it and
existing entries below it must be real paths, not symlinks, because
runner-backed commands rewrite files there. Concurrent runner-backed commands
for the same workspace serialize access to that shared `.es-fluent` runner
cache. A dry run that previews pending generation work exits successfully; use
`status` or `check` when a script should fail on pending generated FTL changes.
`watch` runs generation repeatedly in a TUI when Rust source, crate-local
`Cargo.toml`, crate-local `build.rs`, crate-local `i18n.toml`, or the workspace
root `Cargo.toml` or `Cargo.lock` changes. It accepts `--mode`, but not
`--dry-run` or `--force-run`, always writes generation output, and performs the
same generation path setup checks before the TUI opens. File-valued paths such
as `i18n` or `i18n/en`, directory-valued fallback `.ftl` paths, and invalid
generated `i18n.rs` module setup are rejected before runner metadata is
prepared. `watch` ignores `.ftl` changes so generated writes do not trigger a
loop. For crates whose library target lives at the crate root, top-level
`.es-fluent` and `target` output is also ignored.
Generation, clean, and check fail when two derived items produce the same FTL key in the same output file; rename one item, override one enum variant key, skip a generated item, or split outputs with namespaces.
For namespaced types, check validates the expected namespace file; a key in `{crate}.ftl` still counts as missing if the Rust type belongs in `{crate}/{namespace}.ftl`.

Validate locale setup and Rust/FTL alignment:

```sh
cargo es-fluent check --all
```

With `--all`, check reports non-fallback messages that are still identical to
the fallback locale as untranslated warnings. For intentionally invariant text
such as product names, package names, or keyboard keys, add
`# es-fluent: same-as-fallback` before that message. The marker applies to the
next message entry in that file, so blank lines or additional `#` comments
between the marker and message do not cancel it. To disable the warning for a
crate, set `check_fallback_copies = false` in `i18n.toml`; to disable it for
one all-locale run, pass `--no-fallback-copy-check`.
Passing `--no-fallback-copy-check` without `--all` is rejected before workspace
discovery and is reported as a command error in JSON output.
With `--all`, check also reports orphaned FTL files in discovered non-fallback
locales when those files have no matching file in the fallback locale; valid
crates can still report orphaned files when another selected crate has setup
errors.
Setup validation runs even without `--all`: check reports setup errors such as
a missing or file-valued `assets_dir`, a missing, file-valued, or symlinked
fallback locale directory, locale-looking asset paths that are files or
symlinks instead of real directories,
directory-valued FTL paths such as
`my-crate.ftl`, and non-canonical locale directory names like `i18n/en-us`.
Check also reports symlinked or escaping Cargo library target paths, generated
`i18n.rs` paths that are symlinks or not files, and library `i18n`
declarations that are inline, non-public, or missing when a generated module
file exists. Crates with setup errors are reported before runner
collection for those crates; if every selected crate has setup errors, check
exits with those validation errors without preparing `.es-fluent` runner
metadata or Cargo build output. Setup-invalid crates are excluded from the
temporary runner crate, so unrelated Rust compile errors in those crates do not
hide valid-crate validation issues. JSON `crates_checked` counts only crates
that reached inventory collection and FTL validation.
In JSON mode, discovery, command-level, and validation failures are reported in
the `issues` list and the command exits non-zero when any issue is present.
Command-level failures, such as an unknown crate passed to `--ignore`, use
`kind: "command_error"`. File paths in issue help text are workspace-relative
when they are under the selected workspace root.
Check reports a crate with `i18n.toml` but no Cargo library target as a
validation error, because the CLI inventory runner cannot collect derives from
that crate.
If package filters or `--ignore` leave no crates to validate, check reports
that as a workspace warning instead of a validation issue and exits
successfully.
The GitHub Action runs `cargo es-fluent check`, so a `package` value that
matches no configured crate also exits successfully there; add a separate
`cargo es-fluent status --package <NAME>` step when package-filter typos should
fail CI. Set the Action input `no_fallback_copy_check: true` with `all: true`
to pass `--no-fallback-copy-check` through CI.
After at least one configured crate is selected, names passed to `--ignore` are
validated against configured crates in the workspace, even when `--package`
narrows the crates being checked. If `--package` matches no configured crate,
`check` reports that package-filter warning before validating `--ignore`.
`--ignore` accepts repeated values or comma-separated lists. Quote comma lists
that include spaces, such as `--ignore "api, web"`. Empty comma entries, such
as `--ignore api,`, are rejected.

Run a pre-commit status:

```sh
cargo es-fluent status --all
```

Use `--all` when status should include non-fallback locale formatting, sync,
orphan-file, and validation checks.
Status does not edit project source or locale files, but it may prepare
`.es-fluent` runner metadata and Cargo build output while collecting generation
and validation state for valid crates. Use `--force-run` to bypass the
staleness cache for the runner-backed generation preview and validation pass.
Setup validation runs even without `--all`: status reports setup errors such as
configured `assets_dir` paths that are missing, files, or symlinks instead of
real directories, locale-looking asset paths that are files or symlinks instead
of real directories, missing, file-valued, or symlinked fallback locale
directories, directory-valued FTL paths such as `my-crate.ftl`, and
non-canonical locale directory names such as `i18n/en-us`, plus symlinked or
escaping Cargo library target paths, generated `i18n.rs` paths that are
symlinks or not files, and library `i18n` declarations that are inline,
non-public, or missing when a generated module file exists.
The JSON field `generation_stale_crates` counts crates whose generation dry run
would change at least one file.
Sync or orphan-file checks can also add setup-style errors, such as parse errors
that prevent syncing a non-fallback locale.
The status sync count reports affected crate-locale targets, so the same locale
needing sync in two workspace crates counts as `2`.
When initial setup errors are present, status skips generation, formatting, sync,
orphan-file, and validation checks that depend on valid locale paths.
Status exits non-zero when no crates with `i18n.toml` are discovered, so a
wrong CI path or package filter does not look clean.
Status also exits non-zero when a configured crate has no Cargo library target;
that is reported as a setup error. Empty selections and initial setup errors
are reported before status prepares runner metadata or Cargo build output.
Workspace-level issues, such as a package filter that matches no configured
crate, are included as workspace warnings in the status report.
In JSON mode, discovery and setup failures are reported in the `setup_errors`
list and the command exits non-zero. File paths in `setup_errors`,
`generation_errors`, `format_errors`, and `orphaned_files` are
workspace-relative when they are under the selected workspace root. Orphaned FTL
files do not also increment `validation_errors`. Validation warnings, such as
untranslated fallback-copy warnings from `--all`, increment
`validation_warnings` and also make `clean: false`. Use `--all` to include
non-fallback locale formatting, sync, orphan-file, and validation checks beyond
setup validation.

Format FTL:

```sh
cargo es-fluent fmt --all
```

`cargo es-fluent format` is accepted as an alias; `fmt` is the canonical form
shown in help and examples.

`fmt --dry-run` previews changes and exits successfully when files would be
formatted; use the text summary or JSON `formatted_count` if a script should
treat previewed formatting work as a failure.
`fmt` is crate-layout scoped: it formats `assets_dir/{locale}/{crate}.ftl` and
`assets_dir/{locale}/{crate}/{namespace}.ftl` files for selected crates. Other
FTL files in the same locale directories are ignored by `fmt`; use
`status --all`, `check --all`, or `clean --orphaned --dry-run` to audit
orphaned or unrelated FTL files. Use `--all` to apply formatting to that
selected-crate layout in every discovered locale directory.
The configured `assets_dir` and fallback locale path must exist as real
directories, not symlinks, before formatting. With `--all`, locale-looking
asset paths must also be real directories; files such as `i18n/fr` are reported
as format errors, and non-canonical locale directory names such as `i18n/en-us`
are reported instead of being formatted.
Discovered FTL paths must be real files, not symlinks; a directory named like
`my-crate.ftl` or `my-crate/ui.ftl` is reported as a setup error instead of
being formatted.
In JSON mode, crate-level failures that prevent file enumeration, such as
invalid config paths or unreadable FTL layouts, are reported in the `errors`
list. File paths in `files` and `errors` are workspace-relative when they are
under the selected workspace root. Path-specific setup or format failures, such
as file-valued locale directories or parse errors in a discovered FTL file, stay
attached to entries in `files`. For mixed workspaces, successful file entries
may still be included when another selected crate has file-level format errors.
`fmt` is not transactional across files or crates: in non-dry-run mode, files
that can be formatted may already be written even if another file later fails
and the command exits non-zero.

Sync fallback keys to non-fallback locales:

```sh
cargo es-fluent sync --all
cargo es-fluent sync --locale fr-FR,zh-CN
cargo es-fluent sync --locale "fr-FR, zh-CN"
```

Explicit sync targets must not be the configured fallback locale. `--all` and
`--locale` are mutually exclusive, and `--create` requires explicit `--locale`
targets. Target-selection errors are reported before workspace discovery and,
with `--output json`, are emitted as JSON. Without `--create`, each explicit
target locale directory must already exist for every selected crate. Existing
target locale paths must be real directories, not symlinks. Duplicate explicit
locale targets are ignored. When fallback files use namespaces, matching target
parent paths must also be directories; a file
such as `i18n/fr/my-crate` blocks creation of `i18n/fr/my-crate/ui.ftl` and is
reported before any same-crate file is written. Existing target FTL paths must
be real files, not symlinks or directories, and are checked before any
same-crate file is written. Quote comma lists that include spaces. Empty comma
entries, such as `--locale fr-FR,`, are rejected.
Explicit target runs do not scan unrelated locale directories; use `--all`,
`status --all`, or `doctor` to audit discovered locale directory names. When
`assets_dir = "."`, crate-root project directories such as `bin` are
intentionally ignored by those all-locale audits.
Use `--create` only with explicit `--locale` targets; it cannot be combined
with `--all` because `--all` only processes locale directories that already
exist. When `assets_dir = "."`, `--create` rejects missing target locale names
that match crate-root project directories ignored by all-locale scans, such as
`bin`. With `--all`, locale-looking asset paths must also be real directories;
files such as `i18n/fr` are reported as setup errors instead of being skipped.
The configured `assets_dir` must exist as a real directory for explicit `--locale`
runs and `--all`.
With `--create`, missing target locale directories are created even when the
fallback locale directory has no FTL files yet. The fallback locale directory
itself must exist as a directory. `sync` exits non-zero if no crate is
selected, such as when `--package` matches no configured crate.
Use `--dry-run` to preview locale directories and keys that would be synced
without writing them.
With `--dry-run --create`, JSON `locale_created: true` and `locales_affected`
preview locale directory creation even when `keys_added` is `0`; no directory
is written during the dry run. `locales_affected` counts affected crate-locale
targets, so the same locale changed in two workspace crates counts as `2`.
In JSON mode, sync discovery, setup, and target-locale failures are reported in
the `errors` list and the command exits non-zero. Error paths under the selected
workspace root are workspace-relative. Before writing, `sync` preflights
every selected crate for setup errors, fallback parse errors, and target path
or parse errors. Those failures are reported before any selected crate is
changed. The command is still not transactional for unexpected write-time I/O
failures after preflight succeeds.

Add locale directories seeded from fallback:

```sh
cargo es-fluent add-locale fr-FR zh-CN
cargo es-fluent add-locale fr-FR,zh-CN
cargo es-fluent add-locale "fr-FR, zh-CN"
```

`add-locale` is a text-mode convenience wrapper around
`sync --create --locale <LANG>` for each requested locale. Use
`sync --create --locale <LANG> --output json` when scripts need
machine-readable locale creation results. Locale arguments can be passed
separately or comma-separated. Quote comma lists that include spaces. Empty
comma entries, such as `add-locale fr-FR,`, are rejected. Duplicate locale
targets are ignored. Requested locales must not be the configured fallback
locale. Existing target locale paths must be real directories, not symlinks.
When fallback files use namespaces, matching requested-locale parent paths must
also be directories before seeding starts. Existing requested-locale FTL paths
must be real files, not symlinks or directories.
Unrelated existing locale directories are not scanned by `add-locale`; run
`status --all` or `doctor` when you want to audit discovered locale directory
names. When `assets_dir = "."`, crate-root project directories such as `bin`
are intentionally ignored by those audits, and `add-locale` rejects missing
requested locale directories whose names match those ignored directories.
Use `--dry-run` to preview locale directories and keys that would be added.
If the fallback locale directory exists as a directory but has no FTL files yet,
`add-locale` still creates the requested locale directories; FTL files are
written after fallback keys exist. In `--dry-run` mode, that empty-fallback
case previews locale directory creation and reports `0` keys added. When
requested locales already exist and no fallback keys are missing, `add-locale`
exits successfully and reports that no locale directories or keys needed to be
added. If `--package` matches no configured crate, `add-locale` exits non-zero
instead of reporting success without creating the locale. Like `sync --create`,
`add-locale` preflights every selected crate for setup errors, fallback parse
errors, and requested-locale path or parse errors before writing any selected
crate. The command is still not transactional for unexpected write-time I/O
failures after preflight succeeds.
The configured `assets_dir` and fallback locale path must exist as real
directories, not symlinks, before requested locale directories are created.

Remove stale generated entries and non-fallback file orphans:

```sh
cargo es-fluent clean --all
```

Existing `assets_dir` and fallback locale paths must be real directories, not
symlinks, before clean runs; files such as `i18n` or `i18n/en` are reported as
setup errors.
Existing fallback `.ftl` paths must be real files, not symlinks;
directory-valued FTL paths are reported before runner-backed clean starts.
Runner-backed clean uses the same generated `i18n.rs` module setup checks as
generation before runner metadata or Cargo build output is prepared.
Because `clean --all` also runs the orphan-file scan, it verifies non-fallback
locale paths before runner-backed clean starts.
With `--dry-run`, clean previews locale-file changes and orphan removals
without editing FTL files, but it may still prepare `.es-fluent` runner
metadata and Cargo build output while collecting Rust inventory.

Run an explicit file-orphan scan after the normal clean request:

```sh
cargo es-fluent clean --orphaned
```

Orphan scans compare non-fallback locale files against the fallback locale. The
fallback locale path must exist as a real directory, not a symlink; otherwise
the CLI refuses to scan because every non-fallback file would appear orphaned.
The configured `assets_dir` must also exist as a real directory.
When an orphaned file is removed, now-empty parent directories under that locale
are pruned as well. Orphaned file paths are printed relative to the selected
workspace root when possible, so the locale directory is visible in output.
When `--package` selects one crate, orphan detection still treats files for
other configured workspace crates as expected when they share a locale root.
Locale-looking asset paths must also be real directories; files such as
`i18n/fr` are reported as setup errors instead of being skipped.
Directory-valued FTL paths such as `my-crate.ftl` or `my-crate/ui.ftl` are
also setup errors and are reported before runner-backed clean starts.
Before runner-backed clean starts or any orphan is removed, `clean --orphaned`
verifies that the orphan scan can safely use the configured `assets_dir`,
fallback locale directory, and expected fallback FTL files for configured
crates.
If that setup is invalid, the command exits without preparing `.es-fluent`
runner metadata, Cargo build output, or deleting orphaned files.
For crates with a Cargo library target, `clean --orphaned` still runs the
runner-backed clean first; by default that clean targets the fallback locale,
and with `--all` it targets all discovered locale directories before the orphan
scan starts. If inventory collection or runner execution fails, the explicit
orphan scan is not reached.
When no selected crate has a library target, `clean --orphaned` without `--all`
skips runner-backed clean and the file-only orphan scan can still run. Combining
`--all` with `--orphaned` keeps the normal all-locale generated-key cleaning
requirement, so selected crates must have library targets.

Inspect layout and message IDs:

```sh
cargo es-fluent tree --all
```

Use `--no-attributes` to hide message and term attributes, and
`--no-variables` to hide the Fluent variables referenced by each entry. When
attributes are hidden, variables that occur only inside hidden attributes are
hidden too.

Tree is crate-layout scoped: it inspects `assets_dir/{locale}/{crate}.ftl` and
`assets_dir/{locale}/{crate}/{namespace}.ftl`. Other FTL files in the same
locale directories are ignored; use `status --all`, `check --all`, or
`clean --orphaned --dry-run` to audit orphaned or unrelated FTL files.

The configured `assets_dir` and fallback locale path must exist as real
directories, not symlinks, before tree inspection. With `--all`,
locale-looking asset paths must also be real directories. In JSON mode, tree
discovery and setup failures are reported in the `errors` list and the command
exits non-zero; error paths under the selected workspace root are
workspace-relative. For mixed workspaces,
successful crate trees may still be included in `crates` while failing crates
are listed in `errors`; text output can likewise print earlier crate trees
before a later crate fails.
Empty locale directories are included in the tree even when no FTL files exist
yet.
FTL parse errors inside discovered files are shown in the tree instead of
failing the command; JSON output marks those files with `parse_error: true` and
leaves their `entries` empty. Directory-valued FTL paths such as
`my-crate.ftl` are setup errors, not parse errors, and make the command exit
non-zero. JSON tree output is file-only: `--link-mode` does not collect Rust
source links or prepare runner metadata there. Invalid `--link-mode` values are
reported in the JSON `errors` list when `--output json` is used.

For scripts and editor integrations, prefer `--output json` on commands that
support it. After arguments parse successfully, JSON mode writes only the
command report to stdout, so callers can parse stdout directly and use the exit
status to detect failures. Some successful reports still carry warnings, such
as check workspace warnings or doctor warning issues, so scripts that care
about warning-only states should also inspect those fields. Some argument
problems are validated by commands and included in their JSON reports, such as
check command errors, sync target-selection errors, and invalid
`tree --link-mode` values. Parser-level failures, such as unknown flags,
missing option values, or invalid `--output` values, are reported by clap before
the command runs and use clap's normal stderr help text.
Check and status reports keep workspace-level warnings separate from validation
errors and warnings.
Fmt reports per-file failures in each affected `files[].error` entry and also
lists those failures in the top-level `errors` array.
For commands that support both `--dry-run` and JSON output, such as `fmt` and
`sync`, the report includes `dry_run: true`; changed, created, or added counts
then describe the previewed work rather than files written to disk. Previewed
work is not itself a command failure; check the counts when a script should fail
on pending dry-run changes.

In hyperlink-capable terminals, `tree` labels can be opened at their crate,
locale, FTL file, entry, attribute, or variable source location. Message and
variable rows link to Rust source by default; use `--link-mode ftl` for FTL
source links. In hyperlink-capable terminals, the default Rust link mode may
prepare `.es-fluent` runner metadata and Cargo build output to collect Rust
source locations for selected crates that have library targets. Crates without
library targets still render from their discovered FTL files, but their message
and variable rows fall back to FTL links or plain labels because there is no
Rust inventory to link. Tree setup errors, such as directory-valued FTL paths
and symlinked or escaping Cargo library target paths, generated `i18n.rs` paths
that are symlinks or not files, and invalid library `i18n` declarations, are
reported before that Rust-link collection starts. If setup is valid but the
selected library cannot compile, Rust-link collection can fail before rendering
the tree. Use `--link-mode ftl` for file-only text inspection because it uses
only discovered FTL files.

Diagnose setup:

```sh
cargo es-fluent doctor
```

`doctor` only inspects project files; it does not prepare `.es-fluent` runner
metadata or Cargo `target` output.

`doctor` reports configured `assets_dir` paths that are missing, files, or
symlinks instead of real directories. It also reports locale-looking asset
paths that are files or symlinks instead of real directories, including
non-canonical names like `i18n/en-us`, because those paths block future sync or
add-locale operations.
It also reports directory-valued FTL paths such as `my-crate.ftl`, because
commands cannot parse or format directories as Fluent files.
When a crate has no library target, `doctor` reports that first and skips
library-module and build-script follow-up checks until a library target exists.
When a crate has a library target, `doctor` reports symlinked or escaping
library target paths as errors before following the generated module path.
It warns when the expected generated `i18n.rs` module is missing unless the
library target directly calls `define_i18n_module!()`, and reports an error
when that path exists as a symlink or is not a file. If a library target
directly calls `define_i18n_module!()` and an unused `src/i18n.rs` also exists,
remove the unused file or move the macro there and declare it with
`pub mod i18n;`.
`doctor` also reports an error when the library target defines inline
`mod i18n { ... }` or declares `i18n` without public visibility. When the
generated module file exists, `doctor` reports an error if the library target
does not declare it as a public external module with `pub mod i18n;`.
It reports a warning when the declared generated module does not invoke
`define_i18n_module!()`.
Warning-only reports, such as direct or aliased manager dependency mismatches,
declared generated modules that do not invoke `define_i18n_module!()`, or
missing build-script asset tracking for generated modules or direct
library-target macro calls that invoke `define_i18n_module!()`, exit
successfully; use the warning count when a script should fail on those
findings. When `doctor` reports missing build-script
tracking in an existing project, add
`es_fluent_build::track_i18n_assets();` to `build.rs` directly and add
`es-fluent-build` under `[build-dependencies]` so existing scaffold files are
not refreshed by another `init` run. If `build.rs` exists but is not a file,
or if it is a symlink, `doctor` reports an error because the local crate should
own a real build script.
`doctor` also warns when an existing tracking call lacks that build dependency.
In JSON mode, discovery and setup failures are reported in the `issues` list
and the command exits non-zero when any error issue is present. File paths in
issue help text are workspace-relative when they are under the selected
workspace root.
If `--package` matches no configured crate, `doctor` reports a warning and exits
successfully because it did not find an error in a selected crate.

Commands accept `--path <PATH>`/`-p <PATH>` when run from a workspace. The path
must be an existing root, manifest, or path inside a crate and must not be empty
or only whitespace; omit `--path` to use the current directory. Use it with
`init` to target a member crate root or that member's `Cargo.toml`; virtual
workspace roots and manifests are rejected, and `init` does not accept
`--package`. For non-`init` commands, a
workspace-root `--path` processes all configured crates when it points to a
workspace root or that root's `Cargo.toml`; a workspace-member root, that
member's `Cargo.toml`, or a path inside that member defaults to that member. Use
`--package <NAME>`/`-P <NAME>` with non-`init` commands to process a specific
configured package from a workspace. `--package` is workspace-wide: when it is
present, it selects the named configured package even if `--path` points inside
a different workspace member. The package filter must not be empty or only
whitespace; surrounding whitespace is trimmed, and you can omit `--package` to
process the default selection. If the selected member or workspace subdirectory
has no `i18n.toml`, the command sees an empty es-fluent selection rather than
falling back to sibling crates. `generate`, `watch`, `clean`, `fmt`, `sync`,
`add-locale`, `tree`, and `status` exit non-zero when `--package` matches no
configured crate, so package-filter typos do not look successful. `check` and
`doctor` report that case as a workspace warning and still exit successfully
unless they find an actual issue. Filtered commands discover and parse only the
selected package or member, so an invalid `i18n.toml` in an unselected sibling
does not block the run. Runner-backed filtered commands also link only the
selected package, so unrelated workspace crates do not need to compile for a
package-scoped run. Workspace-root runs still parse every configured workspace
crate. Runner-backed commands for the same workspace serialize access to the
shared `.es-fluent` runner cache, so concurrent invocations may wait for one
another.

## Generated Layout

Without namespaces, generated fallback messages go to:

```text
assets_dir/{fallback_language}/{crate}.ftl
```

With namespaces:

```text
assets_dir/{fallback_language}/{crate}/{namespace}.ftl
```

Discovered non-fallback locale directories mirror the fallback layout after
`sync --all` or `add-locale`.
