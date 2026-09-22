# es-fluent-shared

[![crates.io: es-fluent-shared][crate-badge]][crate]

Runtime-safe metadata, naming, error, locale, and asset-path types shared by the
`es-fluent` facade, derives, managers, generators, and CLI.

Applications should depend on [`es-fluent`][es-fluent].
Use this crate directly only when custom tooling or runtime integration needs
the workspace's shared typed metadata without proc-macro code.

[es-fluent]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent/README.md
[crate-badge]: https://img.shields.io/crates/v/es-fluent-shared.svg?label=es-fluent-shared
[crate]: https://crates.io/crates/es-fluent-shared
