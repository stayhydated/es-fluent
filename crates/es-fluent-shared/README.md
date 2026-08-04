# es-fluent-shared

Runtime-safe metadata, naming, error, locale, and asset-path types shared by the
`es-fluent` facade, derives, managers, generators, and CLI.

Applications should depend on [`es-fluent`](../es-fluent/README.md).
Use this crate directly only when custom tooling or runtime integration needs
the workspace's shared typed metadata without proc-macro code.
