# CLI workflow

Read this reference when generating, validating, formatting, synchronizing,
inspecting, or cleaning FTL resources.

## Prerequisites

- Install with `cargo install es-fluent-cli --locked`.
- Put `i18n.toml` beside the owner package's
  `Cargo.toml`.
- Create the fallback locale directory.
- Keep localizable types in a library target.

Examples use `cargo es-fluent <COMMAND>`. Direct
`cargo-es-fluent` invocation is equivalent.

## Routine workflow

After changing derived types:

~~~sh
cargo es-fluent generate
cargo es-fluent status --all-locales
cargo es-fluent check --all-locales
~~~

Generation uses conservative mode by default: it adds derived entries and
updates their variables while preserving existing translations. Preview
aggressive regeneration before using it:

~~~sh
cargo es-fluent generate --mode aggressive --dry-run
~~~

`status` is non-mutating and summarizes pending generation, cleanup,
formatting, sync, orphan, and validation work.

## Manage locales

~~~sh
cargo es-fluent add-locale fr-FR
cargo es-fluent sync --all-locales
cargo es-fluent fmt --all-locales
~~~

Use `sync --locale <LANG> --create` when explicit locale creation
needs the `sync` JSON surface. `--create` and
`--all-locales` are mutually exclusive.

All-locale validation warns when translated text still matches fallback text.
Mark intentionally invariant entries:

~~~ftl
# es-fluent: same-as-fallback
product-name = es-fluent
~~~

## Clean carefully

`clean` removes entries absent from derive inventory, including
manual-only entries, and can remove empty package-owned files. Always preview:

~~~sh
cargo es-fluent clean --all-locales --dry-run
cargo es-fluent clean --orphaned --dry-run
~~~

`--orphaned` finds non-fallback files without a matching fallback
resource.

## Inspect and automate

~~~sh
cargo es-fluent tree
cargo es-fluent tree --link-mode ftl
cargo es-fluent tree --output json
~~~

Text `tree` can link to Rust or FTL source. JSON output does not
accept `--link-mode`.

`check`, `fmt`, `sync`, `tree`, and
`status` support `--output json`. Use the exit status plus
the report fields relevant to warnings or dry-run work.

Commands that write FTL plan the selected change before committing it and roll
back earlier writes if the command fails.

## Select workspace scope

- `--path <PATH>` selects a crate, workspace, manifest, or path
  inside a member.
- A workspace-root path processes all configured packages.
- A member path selects that member.
- `--package <NAME>` selects one configured package.
- `check --ignore <NAME>` excludes packages and cannot be combined
  with `--package`.

A filter selecting no configured package exits non-zero. Package-scoped runs do
not analyze unrelated sibling derive inventory.

The Cargo package name is the default package-local domain. Additional
`domains` remain owned by the declaring package. Namespaces split a
domain into files; they do not change ownership. The same domain and ID may
appear in another package.
