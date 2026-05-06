[![Docs](https://docs.rs/es-fluent-cli-helpers/badge.svg)](https://docs.rs/es-fluent-cli-helpers/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-cli-helpers.svg)](https://crates.io/crates/es-fluent-cli-helpers)

# es-fluent-cli-helpers

**Internal Crate**: Runtime companion for the generated `.es-fluent/` runner
binary.

The `es-fluent` CLI generates a temporary runner workspace that links a user's
library crates so their inventory registrations become visible at runtime.
`es-fluent-cli-helpers` is the library that binary calls into after decoding a
serialized [`RunnerRequest`](../es-fluent-runner/README.md).

## What it handles

- `generate`: build an `EsFluentGenerator`, validate namespace policy, and write
  `result.json`
- `clean`: run the generator's clean flow and write `result.json`
- `check`: collect expected keys from inventory and write `inventory.json`

Commands that operate directly on existing `.ftl` files such as `fmt`,
`sync`, and `tree` stay in [`es-fluent-cli`](../es-fluent-cli/README.md) and do
not go through this crate.
