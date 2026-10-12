# es-fluent-toml

[![Codecov: es-fluent-toml][codecov-badge]][codecov]
[![crates.io: es-fluent-toml][crate-badge]][crate]

Parser and path resolver for package-local `i18n.toml` configuration. Custom
localization tooling can depend on this crate to validate fallback locales,
asset paths, namespaces, domains, and missing-message or fallback-copy policies.
Applications normally use it through [`es-fluent-cli`][cli], manager macros,
or [`es-fluent-build`][build].

Requires Rust 1.99 or newer.

## Example

Given a package directory containing `Cargo.toml`, `i18n.toml`, and the configured
locale asset tree, load its resolved layout:

~~~rust,no_run
use es_fluent_toml::{I18nConfigError, ResolvedI18nLayout};
use std::path::Path;

fn inspect(package_dir: &Path) -> Result<(), I18nConfigError> {
    let layout = ResolvedI18nLayout::from_manifest_dir(package_dir)?;
    println!("Fallback locale: {}", layout.fallback_language());
    Ok(())
}
~~~

The loader validates the configuration and resolves asset paths relative to
that package, returning an error for invalid inputs.

[cli]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent-cli/README.md
[build]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent-build/README.md
[codecov-badge]: https://codecov.io/gh/stayhydated/es-fluent/branch/master/graph/badge.svg?component=es-fluent-toml
[codecov]: https://codecov.io/gh/stayhydated/es-fluent
[crate-badge]: https://img.shields.io/crates/v/es-fluent-toml.svg?label=es-fluent-toml
[crate]: https://crates.io/crates/es-fluent-toml
