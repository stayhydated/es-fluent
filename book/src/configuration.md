# Configure a project

Each package that owns localizable types uses an `i18n.toml` beside
its `Cargo.toml`. The configuration identifies the fallback locale,
the locale asset root, and optional generation rules.

~~~toml
fallback_language = "en"
assets_dir = "assets/locales"

# Optional Cargo features needed to compile localizable library types.
# fluent_feature = ["my-feature"]

# Optional allowlist for literal namespace values.
# namespaces = ["ui", "errors"]

# Optional additional package-local FTL resources.
# domains = ["emails"]

# Optional: disable warnings for translated values that match fallback text.
# check_fallback_copies = false
~~~

## Fields

| Field | Required | Meaning |
| --- | ---: | --- |
| `fallback_language` | Yes | Canonical BCP-47 tag used for generated fallback resources. |
| `assets_dir` | Yes | Locale asset directory, relative to and contained within the package root. |
| `fluent_feature` | No | Cargo features enabled while the CLI collects derive inventory. |
| `namespaces` | No | Allowlist for literal `namespace = "..."` values. |
| `domains` | No | Additional FTL domains owned by this package. |
| `check_fallback_copies` | No | Enables or disables all-locale warnings for unchanged fallback text. |

Locale directory names and CLI locale arguments must use canonical BCP-47 tags,
such as `en`, `fr-FR`, and `zh-CN`. Use canonical
replacements for deprecated aliases.

The configured asset path must stay inside the package. Existing path
components, locale directories, and discovered FTL paths must have the expected
file type and must not be symlinks. These checks prevent commands from reading
or writing outside the configured locale tree.

## Resource layout

The Cargo package name is the default FTL domain:

~~~text
assets/locales/
├── en/
│   └── my-package.ftl
└── fr-FR/
    └── my-package.ftl
~~~

A custom `[lib] name` or renamed dependency does not change that
default domain.

Namespaces split a domain into nested files:

~~~text
assets/locales/en/my-package/ui.ftl
~~~

See [Namespaces and file splitting](namespaces.md) for the supported namespace
rules.

Additional domains create sibling resources:

~~~toml
domains = ["emails"]
~~~

A type annotated with `#[fluent(domain = "emails")]` keeps its
generated message ID and writes to `emails.ftl`. Do not list the
Cargo package name in `domains`; the default domain is implicit.
Domains belong to the package that declares them and do not reference another
crate.

## Workspaces

Give every package that owns localizable types its own configuration, fallback
resources, and library-reachable manager module. From the workspace root, run:

~~~sh
cargo es-fluent generate --path .
cargo es-fluent status --path . --all-locales
cargo es-fluent check --path . --all-locales
~~~

Each selected package is validated against its own domains and IDs. Different
packages may reuse a domain name or generated ID without colliding.

## Feature-gated messages

When derives are behind Cargo features, list those features in
`fluent_feature` so CLI inventory matches the application build:

~~~toml
fluent_feature = ["admin-ui", "reports"]
~~~

Keep the list package-local. Package-filtered CLI runs compile only the selected
package and its required dependencies.
