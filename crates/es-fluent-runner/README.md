# es-fluent-runner

**Internal Crate**: Shared protocol and filesystem helpers for the temporary
`.es-fluent/` runner workspace created by `es-fluent-cli`.

This crate keeps the contract between the CLI host process, the generated runner
binary, and [`es-fluent-cli-helpers`](../es-fluent-cli-helpers/README.md) in one
place.

## What it provides

- `RunnerRequest` and `RunnerParseMode`: serialized commands sent to the runner
  binary
- `RunnerResult`, `InventoryData`, and `ExpectedKey`: serialized metadata written
  back to disk
- Helpers for `.es-fluent/metadata/{crate}/result.json` and
  `.es-fluent/metadata/{crate}/inventory.json`
- Locale-directory discovery helpers used by runner-backed commands

## Who should use it

You normally should not depend on this crate directly. It exists so
`es-fluent-cli` and `es-fluent-cli-helpers` can share a stable wire format and
filesystem layout without duplicating serde types and path logic.
