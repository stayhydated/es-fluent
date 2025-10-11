# es-fluent-sc-parser

The `es-fluent-sc-parser` (Source Code Parser) crate is responsible for walking through Rust source code directories, parsing files, and extracting translation metadata from types that use the `es-fluent` derive macros.

This is an internal crate, serving as the parsing engine for developer tools like `es-fluent-build` and `es-fluent-cli`. It uses `syn` to parse Rust code into an Abstract Syntax Tree (AST) and a `syn::visit::Visit` implementation to traverse the AST and find relevant items.
