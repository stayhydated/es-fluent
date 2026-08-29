# es-fluent Expo native runtime

This crate is the UniFFI boundary embedded by `@es-fluent/expo`. It consumes the
manifest and resource strings emitted by `cargo es-fluent export typescript`,
builds immutable Project Fluent bundles in Rust, and creates independent locale
requests for React Native roots.

Its public foreign-language surface uses JSON only for generated manifests,
snapshots, and message argument maps. Runtime and request objects remain native
UniFFI objects behind the Expo module's numeric handles.
