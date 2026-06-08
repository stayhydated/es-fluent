[![Docs](https://docs.rs/es-fluent-cli/badge.svg)](https://docs.rs/es-fluent-cli/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-cli.svg)](https://crates.io/crates/es-fluent-cli)

# es-fluent-cli

The official command-line tool for `es-fluent`.

This tool automatically manages your Fluent (`.ftl`) translation files from
derive inventory emitted by your workspace library crates. It finds types with
`#[derive(EsFluent)]`, `#[derive(EsFluentVariants)]`, and
`#[derive(EsFluentLabel)]` and keeps the corresponding message entries in sync.

## Installation

```sh
cargo install es-fluent-cli --locked
```

Examples use Cargo's subcommand form, `cargo es-fluent <COMMAND>`. The
installed binary also accepts direct invocation as
`cargo-es-fluent <COMMAND>`, so `cargo-es-fluent --help`,
`cargo-es-fluent generate --help`, and `cargo-es-fluent help generate` show the
same command surface as `cargo es-fluent --help`.

## Commands

### Common Workspace Options

Commands accept `--path <PATH>`/`-p <PATH>` to choose an existing root,
manifest, or path inside a crate instead of the current directory. The path
value must not be empty or only whitespace; omit `--path` to use the current
directory. Workspace-root `--path` values process all configured crates when
they point to a workspace root or its `Cargo.toml`, but default to the selected
member when `--path` is a workspace member root, that member's `Cargo.toml`, or
a path inside one. If that path includes a symlink inside a member, the member
is selected from the path you passed rather than from the symlink target. Use
`--package <NAME>`/`-P <NAME>` with commands that operate on discovered crates
to process a specific configured package from a workspace. `--package` is
workspace-wide: when it is present, it selects the named configured package even
if `--path` points inside a different workspace member. The package filter must
not be empty or only whitespace; surrounding whitespace is trimmed, and you can
omit `--package` to process the default selection. If the selected member or
workspace subdirectory has no `i18n.toml`, the command sees an empty es-fluent
selection rather than falling back to sibling crates. `generate`, `watch`,
`clean`, `fmt`, `sync`, `add-locale`, `tree`, and `status` exit non-zero when
`--package` matches no configured crate, so package-filter typos do not look
successful. `check` reports that case as a workspace warning and still exits
successfully unless it finds an actual issue. Filtered commands discover and
parse only the selected package or member, so an invalid `i18n.toml` in an
unselected sibling does not block the run. Runner-backed filtered commands also
link only the selected package, so unrelated workspace crates do not need to
compile for a package-scoped run. Workspace-root runs still parse every
configured workspace crate.

### Generate

When you add new localizable structs or enums to your library target, run:

```sh
cargo es-fluent generate
```

This will:

1. Collect derive inventory registrations from workspace library targets.
1. Update `assets_dir/{fallback}/{your_crate}.ftl` (and `assets_dir/{fallback}/{your_crate}/{namespace}.ftl` for namespaced types).
   - **New items**: Added as new messages.
   - **Changed items**: Variables updated (e.g. if you added a field).
   - **Existing translations**: Preserved untouched.

Use `--mode conservative` to merge generated keys while preserving manual-only
entries and existing translations. This is the default. Use `--mode aggressive`
when you want generated files rebuilt from the current Rust inventory.
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

Use `--dry-run` to preview locale-file changes without editing FTL files. Like
`status`, runner-backed dry runs may still prepare `.es-fluent` metadata and
Cargo build output while collecting Rust inventory. If `.es-fluent` already
exists, it and existing entries below it must be real paths, not symlinks,
because runner-backed commands rewrite files there. Concurrent runner-backed
commands for the same workspace serialize access to that shared `.es-fluent`
runner cache. A dry run that previews pending generation work exits
successfully; use `status` or `check` when a script should fail on pending
generated FTL changes. Use `--force-run` to bypass the staleness cache and run
the generated runner through Cargo.

Literal string namespaces are checked as safe relative namespace paths at compile time. If you configure `namespaces = [...]` in `i18n.toml`, string-based namespaces are validated against the allowlist by both the compiler and the CLI during `generate` and `watch`.

### Namespaces (optional)

You can split output into multiple files by annotating types:

```rs
#[derive(EsFluent)]
#[fluent(namespace = "ui")] // -> assets_dir/{locale}/{crate}/ui.ftl
struct Button;

#[derive(EsFluent)]
#[fluent(namespace = file)] // -> assets_dir/{locale}/{crate}/{file_stem}.ftl
struct Dialog;

#[derive(EsFluent)]
#[fluent(namespace = file_relative)] // -> assets_dir/{locale}/{crate}/ui/button.ftl
struct Modal;

#[derive(EsFluent)]
#[fluent(namespace = folder)] // -> assets_dir/{locale}/{crate}/{parent_folder}.ftl
struct FolderModal;

#[derive(EsFluent)]
#[fluent(namespace = folder_relative)] // -> assets_dir/{locale}/{crate}/ui.ftl
struct FolderRelativeModal;
```

### Watch

Run the generation step in a TUI and keep regenerating when Rust source,
crate-local `Cargo.toml`, crate-local `build.rs`, crate-local `i18n.toml`,
or the workspace root `Cargo.toml` or `Cargo.lock` changes:

```sh
cargo es-fluent watch
```

`watch` accepts the same `--mode conservative|aggressive` option as
`generate`, but it does not accept `--dry-run` or `--force-run` and always
writes generation output. The same generation path setup checks apply before
the TUI opens. File-valued paths such as `i18n` or `i18n/en`,
directory-valued fallback `.ftl` paths, and invalid generated `i18n.rs` module
setup are rejected before runner metadata is prepared. Changes to `.ftl` files
are ignored so generated writes do not trigger a loop. For crates whose library
target lives at the crate root, top-level `.es-fluent` and `target` output is
also ignored.

### Check

Validate locale setup and ensure your FTL files match Rust-derived keys and variables:

```sh
cargo es-fluent check
```

Use `--all` to check all discovered locale directories, not just the fallback
language. Use `--ignore <CRATE>` to skip specific crates; it can be repeated or
passed as a comma-separated list. Quote comma lists that include spaces, such
as `--ignore "api, web"`. Empty comma entries, such as `--ignore api,`, are
rejected. Use `--force-run` to bypass the staleness cache.
Fallback-copy warnings are only produced by `--all`; pass
`--no-fallback-copy-check` on an all-locale run to disable them for that run.
Passing `--no-fallback-copy-check` without `--all` is rejected before workspace
discovery and is reported as a command error in JSON output.
FTL variables that are not declared by Rust code are reported as errors.
Rust-declared variables omitted by a translation are reported as warnings; any
reported validation issue makes `check` exit non-zero for CI enforcement.
Crates with `i18n.toml` but no Cargo library target are reported as validation
errors because the CLI inventory runner cannot collect derives from them.
When `--all` checks non-fallback locales, messages that are still identical to
the fallback locale are reported as untranslated warnings. If a value is
intentionally invariant, such as a product name or keyboard key, add
`# es-fluent: same-as-fallback` before that message. The marker applies to the
next message entry in that file, so blank lines or additional `#` comments
between the marker and message do not cancel it. You can also set
`check_fallback_copies = false` in `i18n.toml` for that crate.
With `--all`, `check` also reports orphaned FTL files in discovered
non-fallback locales as errors when those files have no matching file in the
fallback locale; valid crates can still report orphaned files when another
selected crate has setup errors.
Setup validation runs even without `--all`: `check` also reports setup errors
such as a missing or file-valued `assets_dir`, a missing, file-valued, or
symlinked fallback locale directory, locale-looking asset paths that are files
or symlinks instead of real directories,
directory-valued FTL paths such as
`my-crate.ftl`, and non-canonical locale directory names like `i18n/en-us`.
It also reports symlinked or escaping Cargo library target paths, generated
`i18n.rs` paths that are symlinks or not files, and library `i18n`
declarations that are inline, non-public, or missing when a generated module
file exists. Crates with setup errors are reported before runner
collection for those crates; if every selected crate has setup errors, `check`
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
If package filters or `--ignore` leave no crates to validate, `check` reports
that as a workspace warning instead of a validation issue and exits
successfully.
After at least one configured crate is selected, names passed to `--ignore` are
validated against configured crates in the workspace, even when `--package`
narrows the crates being checked. If `--package` matches no configured crate,
`check` reports that package-filter warning before validating `--ignore`.
For namespaced types, `check` validates the expected namespace file, so a key
in `{crate}.ftl` still reports as missing when the Rust type belongs in
`{crate}/{namespace}.ftl`.

### Clean

Remove stale generated keys and groups that are absent from your source code:

```sh
cargo es-fluent clean
```

Use `--dry-run` to preview locale-file changes without editing FTL files. Like
`status`, runner-backed dry runs may still prepare `.es-fluent` metadata and
Cargo build output while collecting Rust inventory. Use `--all` to clean all
discovered locale directories. Use `--force-run` to bypass the staleness cache.
`clean --all` only targets canonical locale directories from the configured
`assets_dir`; invalid or non-canonical locale directory names are reported and
left unchanged.
`clean --all` also removes orphaned FTL files in non-fallback locales when the
file has no matching file in the fallback locale.
Because of that orphan-file scan, `clean --all` verifies non-fallback locale
paths before runner-backed clean starts.
Locale-looking asset paths must be real directories, not symlinks; files such
as `i18n/fr` are reported as setup errors instead of being skipped.
Existing `assets_dir` and fallback locale paths must also be real directories,
not symlinks; files such as `i18n` or `i18n/en` are reported as setup errors
before clean runs.
Existing fallback `.ftl` paths must be real files, not symlinks;
directory-valued FTL paths are reported before runner-backed clean starts.
Runner-backed clean uses the same generated `i18n.rs` module setup checks as
generation before runner metadata or Cargo build output is prepared.

When the main crate file has zero non-namespaced registered Rust types, `clean`
deletes that stale main file. When a namespaced file has zero registered Rust
types, `clean` also removes that stale namespace file in the locale being
cleaned so discovery metadata stays in sync with code.

#### Clean Orphaned Files

Run an explicit orphan-file scan for non-fallback locales after the normal clean
request:

```sh
cargo es-fluent clean --orphaned
```

This compares files in non-fallback locales against the configured fallback
locale (`en` in the executable README example). Files that exist in
non-fallback locales but have no corresponding file in the fallback locale are
considered orphaned and will be removed. The fallback locale itself is never
modified.
When an orphaned file is removed, now-empty parent directories under that locale
are pruned as well. Orphaned file paths are printed relative to the selected
workspace root when possible, so the locale directory is visible in output.
Before runner-backed clean starts or any orphan is removed, `clean --orphaned`
verifies that the orphan scan can safely use the configured `assets_dir`,
fallback locale directory, and expected fallback FTL files for configured
crates.
When `--package` selects one crate, orphan detection still treats files for
other configured workspace crates as expected when they share a locale root.
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
The fallback locale path must exist as a real directory, not a symlink, before
an orphan scan runs; otherwise the CLI refuses to scan because every
non-fallback file would appear orphaned.
The configured `assets_dir` must also exist as a real directory.
Locale-looking asset paths must also be real directories; files such as
`i18n/fr` are reported as setup errors instead of being skipped.
Directory-valued FTL paths such as `my-crate.ftl` or `my-crate/ui.ftl` are
also setup errors and are reported before runner-backed clean starts.

Use `--dry-run` to preview which files would be removed without actually deleting them. Passing `--orphaned` scans non-fallback locale files even when `--all` is not set.

### Fmt

Standardize the formatting of your FTL files using `fluent-syntax` rules:

```sh
cargo es-fluent fmt
```

`cargo es-fluent format` is accepted as an alias; `fmt` is the canonical form
shown in help and examples.

Use `--dry-run` to preview changes and print diffs without writing them. A
dry-run that finds files needing formatting exits successfully; use the text
summary or JSON `formatted_count` if a script should treat previewed formatting
work as a failure. Use `--all` to format the selected crate's generated FTL
layout in all discovered locale directories.
`fmt` is crate-layout scoped: it formats `assets_dir/{locale}/{crate}.ftl` and
`assets_dir/{locale}/{crate}/{namespace}.ftl` files for selected crates. Other
FTL files in the same locale directories are ignored by `fmt`; use
`status --all`, `check --all`, or `clean --orphaned --dry-run` to audit
orphaned or unrelated FTL files.
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

### Sync

Propagate keys from your fallback language to other languages (e.g., from `en` to `fr-FR` and `zh-CN`), creating placeholders for missing translations:

```sh
cargo es-fluent sync --all
cargo es-fluent sync --locale fr-FR,zh-CN
cargo es-fluent sync --locale "fr-FR, zh-CN"
```

Use `--locale <LANG>` to sync specific locales, repeating the flag or passing a
comma-separated list. Quote comma lists that include spaces, such as
`--locale "fr-FR, zh-CN"`. Use `--all` to sync all discovered non-fallback
locale directories.
Empty comma entries, such as `--locale fr-FR,`, are rejected. Duplicate
explicit locale targets are ignored. `--all` cannot be combined with
`--locale`. Running `sync` without either option exits non-zero with an
actionable message. These target-selection errors are reported before workspace
discovery and, with `--output json`, are emitted as JSON. Use `--dry-run` to
preview locale directories and keys that would be synced, and print diffs
without writing them. Use `--create` with
`--locale <LANG>` to create missing locale
directories before seeding them from the fallback locale; `--create` cannot be
combined with `--all` because `--all` only processes locale directories that
already exist. When `assets_dir = "."`, `--create` rejects missing target
locale names that match crate-root project directories ignored by all-locale
scans, such as `bin`. Explicit `--locale` targets must not be the configured
fallback locale. Existing target locale paths must be real directories, not
symlinks, and, without `--create`, each target locale directory must already
exist for every selected crate. When fallback files use namespaces, matching
target parent paths must also be directories; a file such as
`i18n/fr/my-crate` blocks creation of `i18n/fr/my-crate/ui.ftl` and is
reported before any same-crate file is written. Existing target FTL paths must
be real files, not symlinks or directories, and are checked before any
same-crate file is written. Explicit target runs do not scan
unrelated locale directories; use
`--all`, `status --all`, or `check --all` to audit discovered locale directory
names. When `assets_dir = "."`, crate-root project directories such as `bin`
are intentionally ignored by those all-locale audits. With `--all`,
locale-looking asset paths must also be real directories; files such as
`i18n/fr` are reported as setup errors instead of being skipped.
The configured `assets_dir` must exist as a real directory for explicit
`--locale` runs and `--all`.
With `--create`, missing target locale directories are created even when the
fallback locale directory has no FTL files yet. The fallback locale directory
itself must exist as a directory. `sync` exits non-zero if no crate is
selected, such as when `--package` matches no configured crate.
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

The `sync` command properly handles namespaced FTL files, creating matching subdirectories in target locales when syncing from the fallback locale.

### Add Locale

Create one or more locale directories and seed them from the fallback locale:

```sh
cargo es-fluent add-locale fr-FR zh-CN
cargo es-fluent add-locale fr-FR,zh-CN
cargo es-fluent add-locale "fr-FR, zh-CN"
```

This is a text-mode convenience wrapper around `sync --create --locale <LANG>`
for each requested locale. Use `sync --create --locale <LANG> --output json`
when scripts need machine-readable locale creation results. Locale arguments can
be passed separately or comma-separated. Quote comma lists that include spaces.
Empty comma entries, such as `add-locale fr-FR,`, are rejected. Duplicate
locale targets are ignored. Requested locales must not be the configured
fallback locale, and existing target locale paths must be real directories, not
symlinks. When fallback files use namespaces, matching requested-locale parent
paths must also be directories before seeding starts. Existing requested-locale
FTL paths must be real files, not symlinks or directories. Use `--dry-run` to
preview locale directories and keys that would be added. Unrelated existing
locale directories are not scanned by `add-locale`; run `status --all` or
`check --all` when you want to audit discovered locale directory names. When
`assets_dir = "."`, crate-root project directories such as `bin` are
intentionally ignored by those audits, and `add-locale` rejects missing
requested locale directories whose names match those ignored directories.
The configured `assets_dir` and fallback locale path must exist as real
directories, not symlinks, before requested locale directories are created.
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

### Tree

Inspect the discovered FTL file layout and message IDs for a crate:

```sh
cargo es-fluent tree
cargo es-fluent tree --all
```

Use `--all` to show all discovered locale directories instead of just the
fallback language. Use
`--no-attributes` to hide message and term attributes, and `--no-variables` to
hide the Fluent variables referenced by each entry. When attributes are hidden,
variables that occur only inside hidden attributes are hidden too. The
tree is crate-layout scoped: it inspects `assets_dir/{locale}/{crate}.ftl` and
`assets_dir/{locale}/{crate}/{namespace}.ftl`. Other FTL files in the same
locale directories are ignored; use `status --all`, `check --all`, or
`clean --orphaned --dry-run` to audit orphaned or unrelated FTL files. The
configured `assets_dir` and fallback locale path must exist as real directories,
not symlinks, before tree inspection. With `--all`, locale-looking asset paths
must also be real directories. In JSON mode, tree discovery and setup failures
are reported in the `errors` list and the command exits non-zero; error paths
under the selected workspace root are workspace-relative. For mixed workspaces,
successful crate trees may still be
included in `crates` while failing crates are listed in `errors`; text output
can likewise print earlier crate trees before a later crate fails. Empty locale
directories are included in the tree even when no FTL files exist yet. FTL parse
errors inside discovered files are shown in the tree instead of failing the
command; JSON output marks those files with `parse_error: true` and leaves their
`entries` empty. Directory-valued FTL paths such as `my-crate.ftl` are setup
errors, not parse errors, and make the command exit non-zero. JSON output is
file-only: `--link-mode` does not collect Rust
source links or prepare runner metadata there. Invalid `--link-mode` values are
reported in the JSON `errors` list when `--output json` is used. In terminals
that support hyperlinks, text tree labels link to their crate, locale, FTL file, entry,
attribute, or variable source location. Message and variable rows link to Rust
source by default; use `--link-mode ftl` to link those rows to FTL source
locations instead. In hyperlink-capable terminals, the default Rust link mode
may prepare `.es-fluent` runner metadata and Cargo build output to collect Rust
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

### Status

Run a workflow summary before committing or in CI:

```sh
cargo es-fluent status --all
```

`status` reports whether generation would change fallback files, formatting is
needed, non-fallback locales need synced keys, orphaned files exist, or
validation errors or warnings would be reported. It does not edit project source
or locale files, but it may prepare `.es-fluent` runner metadata and Cargo build
output while collecting generation and validation state. Use `--force-run` to
bypass the staleness cache for the runner-backed generation preview and
validation pass. Setup validation runs even without `--all`: it reports setup
errors such as configured `assets_dir` paths that are missing, files, or
symlinks instead of real directories, locale-looking asset paths that are files
or symlinks instead of real directories, missing, file-valued, or symlinked
fallback locale directories, directory-valued FTL paths such as
`my-crate.ftl`, and non-canonical locale directory names such as `i18n/en-us`,
plus symlinked or escaping Cargo library target paths, generated `i18n.rs`
paths that are symlinks or not files, and library `i18n` declarations that are
inline, non-public, or missing when a generated module file exists.
The JSON field `generation_stale_crates` counts crates whose generation dry run
would change at least one file.
Sync or orphan-file checks can also add setup-style errors, such as parse errors
that prevent syncing a non-fallback locale. It exits non-zero when attention is
required.
The sync count reports affected crate-locale targets, so the same locale
needing sync in two workspace crates counts as `2`.
When initial setup errors are present, status skips generation, formatting, sync,
orphan-file, and validation checks that depend on valid locale paths.
It also exits non-zero when no crates with `i18n.toml` are discovered, which
helps catch CI path or package-filter mistakes.
Configured crates without a Cargo library target are reported as setup errors
and also make status non-zero. Empty selections and initial setup errors are
reported before status prepares runner metadata or Cargo build output.
Workspace-level issues, such as a package filter that matches no configured
crate, are included as workspace warnings in the status report.
In JSON mode, discovery and setup failures are reported in the `setup_errors`
list and the command exits non-zero. File paths in `setup_errors`,
`generation_errors`, `format_errors`, and `orphaned_files` are
workspace-relative when they are under the selected workspace root. Orphaned FTL
files do not also increment `validation_errors`. Validation warnings, such as
untranslated fallback-copy warnings from `--all`, increment
`validation_warnings` and also make `clean: false`.
Use `--all` to include non-fallback locale formatting, sync, orphan-file, and
validation checks beyond setup validation.

### Structured Output

Machine-readable output is available for commands intended for CI and editor
integrations:

```sh
cargo es-fluent check --all --output json
cargo es-fluent status --all --output json
```

`--output json` is supported by `check`, `fmt`, `sync`, `tree`, and
`status`.
After arguments parse successfully, JSON mode writes only the command report to
stdout so scripts can parse it directly; use the exit status to distinguish
failing runs from successful runs. Some successful reports still carry warnings, such as `check` workspace
warnings, so scripts that care about warning-only states should also inspect
those fields. Some argument
problems are validated by commands and included in their JSON reports, such as
`check` command errors, `sync` target-selection errors, and invalid
`tree --link-mode` values. Parser-level failures, such as unknown flags,
missing option values, or invalid `--output` values, are reported by clap before
the command runs and use clap's normal stderr help text.
`check` and `status` include workspace-level warnings separately from
validation errors and warnings.
`check` reports command-level failures, such as an unknown `--ignore` crate, as
`issues` entries with `kind: "command_error"`.
`fmt` reports per-file failures in each affected `files[].error` entry and also
lists those failures in the top-level `errors` array.
For commands that support both `--dry-run` and JSON output, such as `fmt` and
`sync`, the report includes `dry_run: true`; changed, created, or added counts
then describe the previewed work rather than files written to disk. Previewed
work is not itself a command failure; check the counts when a script should fail
on pending dry-run changes.

## CI/CD Integration

### GitHub Actions

```yaml
name: es-fluent
on: [pull_request]

jobs:
  es-fluent:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - name: Check FTL files
        uses: stayhydated/es-fluent/crates/es-fluent-cli@v0.16.0
        with:
          path: .
          all: true
```

Inputs:

- `version`: Optional `es-fluent-cli` version to install from crates.io. If omitted, the action installs the CLI from the pinned action ref. Use `latest` to install the newest crates.io release.
- `path`: Existing path to a crate/workspace root, its `Cargo.toml`, or a path inside a crate (passed as `--path`). Default: `.`.
- `package`: Workspace-wide package name filter (passed as `--package`); when set, it selects the named package even if `path` points inside a different member. Default: empty.
- `all`: Include non-fallback validation, fallback-copy warnings, and orphan-file checks. Must be `true` or `false`. Default: `false`.
- `ignore`: Crates to skip during validation (comma- or newline-separated; blank entries are ignored). Default: empty.
- `no_fallback_copy_check`: Disable fallback-copy warnings for all-locale checks (passed as `--no-fallback-copy-check`). Must be `true` or `false`; requires `all: true`. Default: `false`.
- `force_run`: Run the generated runner through Cargo, ignoring the staleness cache. Must be `true` or `false`. Default: `false`.
- `toolchain`: Rust toolchain to install for the action. Default: `stable`.

This action always runs `cargo es-fluent check`. Pin the `uses` ref to a release
tag or commit SHA for reproducible builds. Omit `version` to run the CLI from
that ref, or set `version` when you intentionally want a crates.io release.
Because the action runs `check`, a `package` value that matches no configured
crate is reported as a workspace warning and exits successfully. Use
`cargo es-fluent status --package <NAME>` in a separate workflow step when a
package-filter typo should fail CI.

## Limitations

The CLI runner links workspace crates as **library targets only**. If you define
`#[derive(EsFluent*)]` types exclusively in a binary target, they won't be registered in the
inventory. Commands that need inventory collection, such as `generate`,
`watch`, `clean`, `check`, and `status`, fail when a configured crate has no
library target. `clean --orphaned` without `--all` can still run its file-only
orphan scan for such crates, and skips runner preparation when no selected
crate has a library target. `clean --all --orphaned` still requires library
targets because the all-locale generated-key clean must run before the orphan
scan. File-only commands such as `fmt`, `sync`, and `add-locale`, plus
`tree --output json` and text `tree --link-mode ftl`, can still inspect or edit
discovered FTL files. Text `tree --link-mode rust` also renders crates without
library targets from discovered FTL files, but only crates with library targets
can contribute Rust source links. Commands that print the shared discovery
summary, such as `fmt` and `clean --orphaned`, report the missing library
target as a notice.
When a custom `[lib].path` exists, runner-backed commands require the existing
library target path components to stay inside the crate and avoid symlinks;
violations are reported before `.es-fluent` runner metadata or Cargo build
output is prepared. Runner-backed commands for the same workspace serialize
access to the shared `.es-fluent` runner cache, so concurrent invocations may
wait for one another.

Workarounds:

- Add a `lib.rs` target and move derived types into it.
- Move shared localization types into a small library crate and depend on it from your binary.
