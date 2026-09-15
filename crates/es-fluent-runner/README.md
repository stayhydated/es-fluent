# es-fluent-runner

[![CI](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml/badge.svg)](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml)
[![Codecov](https://codecov.io/gh/stayhydated/es-fluent/branch/master/graph/badge.svg)](https://codecov.io/gh/stayhydated/es-fluent)
[![Book](https://img.shields.io/badge/book-online-blue)](https://stayhydated.github.io/es-fluent/book/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-runner.svg)](https://crates.io/crates/es-fluent-runner)

Shared request, result, inventory, and filesystem transaction types for the
temporary runner workspace created by `es-fluent-cli`.

This crate is intended for `es-fluent-cli`,
`es-fluent-cli-helpers`, and compatible runner integrations.
Application developers should use
[`cargo es-fluent`](../es-fluent-cli/README.md).
