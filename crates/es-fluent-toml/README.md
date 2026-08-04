# es-fluent-toml

[![Docs](https://docs.rs/es-fluent-toml/badge.svg)](https://docs.rs/es-fluent-toml/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-toml.svg)](https://crates.io/crates/es-fluent-toml)

Parser and path resolver for package-local `i18n.toml` configuration.
It validates fallback locales, asset paths, feature lists, namespace
allowlists, additional package-local domains, and fallback-copy policy.

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

See [Configure a project](https://stayhydated.github.io/es-fluent/book/configuration.html)
for the public file format and resource layout.
