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

Builds `web/public/llms.txt` and `web/public/llms-full.txt` from the current
mdBook sources.

```bash
cargo xtask build llms-txt
```

### `build web`

Builds the Dioxus site for GitHub Pages into `web/dist`.

```bash
cargo xtask build web
```
