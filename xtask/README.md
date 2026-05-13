# xtask

Internal task runner for repository maintenance tasks.

This is for maintainers working inside this repository. Application users should use the published `es-fluent` crates and public book instead of invoking `xtask` directly.

## Commands

### `build bevy-demo`

Builds the Trunk-hosted Bevy demo into `web/public/bevy-demo`.

```bash
cargo xtask build bevy-demo
```

### `build book`

Builds the mdBook into `web/public/book`.

```bash
cargo xtask build book
```

### `build llms-txt`

Builds `web/public/llms.txt`, `web/public/llms-full.txt`, and per-chapter
Markdown files under `web/public/llms/` from the current mdBook sources.

```bash
cargo xtask build llms-txt
```

### `build web`

Builds the release SSG Dioxus site for GitHub Pages into `web/dist`.

```bash
cargo xtask build web
```

### `release plan`

Prints the crates.io publish order for publishable workspace crates. The order
is computed from non-dev workspace dependencies so dependent crates are not
packaged before their registry dependencies exist.

```bash
cargo xtask release plan
```

### `release publish`

Prints the publish commands in release order. By default, this uses
`cargo hack --no-dev-deps publish` because versioned dev-dependencies are
validated during packaging and this workspace has dev-dependency back-references
between crates. Add `--execute` to upload. Use `--from <package>` to resume
after a failure or crates.io index delay.

```bash
cargo xtask release publish
cargo xtask release publish --execute --skip-existing
```

This command requires `cargo-hack` unless you pass `--include-dev-deps`.
