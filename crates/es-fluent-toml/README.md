# es-fluent-toml

[![CI](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml/badge.svg)](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml)
[![Codecov](https://codecov.io/gh/stayhydated/es-fluent/branch/master/graph/badge.svg)](https://codecov.io/gh/stayhydated/es-fluent)
[![Book](https://img.shields.io/badge/book-online-blue)](https://stayhydated.github.io/es-fluent/book/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-toml.svg)](https://crates.io/crates/es-fluent-toml)

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
