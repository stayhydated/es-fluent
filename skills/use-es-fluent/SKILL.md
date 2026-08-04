---
name: use-es-fluent
description: Use when adding, migrating, debugging, documenting, or reviewing es-fluent localization in Rust applications. Covers choosing embedded, Dioxus, or Bevy managers; configuring i18n.toml; deriving typed messages, labels, variants, and language enums; generating and validating FTL with cargo es-fluent; and composing localization across Cargo workspaces.
---

# Use es-fluent

## Workflow

1. Inspect the relevant `Cargo.toml`, `i18n.toml`, library
   target, locale tree, and existing manager module. Preserve the repository's
   compatible release lines.
2. Choose one concrete runtime manager: embedded for general Rust, Dioxus for
   client/SSR, or Bevy for ECS and reactive UI.
3. Keep localizable types and `define_i18n_module!()` reachable from a
   library target. CLI derive discovery does not inspect binary-only types.
4. Give each package that owns messages its own configuration, fallback FTL,
   manager registration, and build-script asset tracking.
5. Use `EsFluent` for messages, `EsFluentVariants` for
   field or variant metadata, `EsFluentLabel` for type labels, and a
   unit-only `EsFluent` enum for selectors when it should also be a
   message.
6. Generate fallback FTL, translate it, then run the narrow relevant CLI check.
7. Localize through the explicit manager or framework context. Use fallible
   lookup only where the caller intentionally handles missing output.

## Decision rules

- Prefer the `es-fluent` facade and a concrete manager over direct
  dependencies on proc-macro or manager-core crates.
- Use canonical BCP-47 locale directory names.
- Treat the Cargo package name as the implicit package-local FTL domain.
  Additional `domains` do not reference another crate.
- Use namespaces to split one domain into files, not to establish ownership.
- Add `es-fluent-build` under `[build-dependencies]` when
  manager macros scan locale assets at compile time.
- Preserve edited fallback translations in conservative generation mode. Preview
  cleanup and aggressive generation before allowing deletions.
- In workspaces, validate from the root when the task spans multiple owner
  packages; use `--package` for intentionally narrow work.

## References

Load only the reference needed for the request:

- [Public facades and setup](references/public-facades.md): dependencies,
  configuration, release lines, and multi-crate ownership.
- [Runtime managers](references/managers.md): embedded, Dioxus, and Bevy setup
  and context rules.
- [Derive and FTL patterns](references/derive-and-ftl.md): attributes,
  generated IDs and arguments, choices, labels, variants, temporal values,
  namespaces, and domains.
- [CLI workflow](references/cli-workflow.md): generation, checks, locale
  management, cleanup, structured output, and workspace selection.
