# es-fluent-generate

[![CI](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml/badge.svg)](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml)
[![Codecov](https://codecov.io/gh/stayhydated/es-fluent/branch/master/graph/badge.svg)](https://codecov.io/gh/stayhydated/es-fluent)
[![Book](https://img.shields.io/badge/book-online-blue)](https://stayhydated.github.io/es-fluent/book/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-generate.svg)](https://crates.io/crates/es-fluent-generate)

FTL generation, merge, cleanup, and formatting support used by
`es-fluent-cli`.

Application developers should use
[`cargo es-fluent`](../es-fluent-cli/README.md). Depend on this crate
directly only when custom tooling needs the same conservative or aggressive
generation and Fluent formatting behavior as the CLI.
