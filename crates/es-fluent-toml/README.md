# es-fluent-toml

[![Codecov: es-fluent-toml][codecov-badge]][codecov]
[![crates.io: es-fluent-toml][crate-badge]][crate]

Parser and path resolver for package-local `i18n.toml` configuration.
It validates fallback locales, asset paths, feature lists, namespace
allowlists, additional package-local domains, missing-message policy, and
fallback-copy policy.

Most applications use this crate through `es-fluent-cli`, manager
macros, or `es-fluent-build`. Custom tooling can load a resolved
layout directly:

~~~rust,no_run
fn main() -> std::io::Result<()> {
    let _layout = es_fluent_toml::ResolvedI18nLayout::from_manifest_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
    )
    .map_err(|error| std::io::Error::other(format!("{error:?}")))?;
    Ok(())
}
~~~

[codecov-badge]: https://codecov.io/gh/stayhydated/es-fluent/branch/master/graph/badge.svg?component=es-fluent-toml
[codecov]: https://codecov.io/gh/stayhydated/es-fluent
[crate-badge]: https://img.shields.io/crates/v/es-fluent-toml.svg?label=es-fluent-toml
[crate]: https://crates.io/crates/es-fluent-toml
