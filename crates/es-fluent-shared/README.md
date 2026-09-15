# es-fluent-shared

[![CI](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml/badge.svg)](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml)
[![Codecov](https://codecov.io/gh/stayhydated/es-fluent/branch/master/graph/badge.svg)](https://codecov.io/gh/stayhydated/es-fluent)
[![Book](https://img.shields.io/badge/book-online-blue)](https://stayhydated.github.io/es-fluent/book/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-shared.svg)](https://crates.io/crates/es-fluent-shared)

Runtime-safe metadata, naming, error, locale, and asset-path types shared by the
`es-fluent` facade, derives, managers, generators, and CLI.

Applications should depend on [`es-fluent`](../es-fluent/README.md).
Use this crate directly only when custom tooling or runtime integration needs
the workspace's shared typed metadata without proc-macro code.
