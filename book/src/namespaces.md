# Namespaces & File Splitting

By default, all your FTL keys land in a single `{crate}.ftl` file per locale. As a project grows, this gets unwieldy. Namespaces let you route specific types into separate `.ftl` files. Every derive macro (`EsFluent`, `EsFluentThis`, `EsFluentVariants`) supports the same namespace attribute.

## Output Layout

| Declaration    | File path                                     |
| -------------- | --------------------------------------------- |
| No namespace   | `assets_dir/{locale}/{crate}.ftl`             |
| With namespace | `assets_dir/{locale}/{crate}/{namespace}.ftl` |

When namespaces are enabled through the manager macros, the configured
namespace files are the canonical per-locale resources.
`{crate}.ftl` is not part of the namespaced resource plan.

## Namespace Modes

### Explicit String

`namespace = "name"` sets an explicit string namespace.

```rust
use es_fluent::EsFluent;

#[derive(EsFluent)]
#[fluent(namespace = "ui")]
pub struct Button<'a>(pub &'a str);
```

This writes the key to `assets_dir/{locale}/{crate}/ui.ftl`.

### File Stem

`namespace = file` uses the source file's stem as the namespace.

```rust
use es_fluent::EsFluent;

// In src/components/dialog.rs
#[derive(EsFluent)]
#[fluent(namespace = file)]
pub struct Dialog {
    pub title: String,
}
```

A type in `src/components/dialog.rs` maps to namespace `dialog`.

### File Relative

`namespace(file(relative))` uses the file path relative to the crate root, strips `src/`, and removes the extension.

```rust
use es_fluent::EsFluent;

// In src/ui/button.rs
#[derive(EsFluent)]
#[fluent(namespace(file(relative)))]
pub enum Gender {
    Male,
    Female,
    Other(String),
}
```

A type in `src/ui/button.rs` maps to namespace `ui/button`.

### Folder

`namespace = folder` uses the source file's parent folder.

```rust
use es_fluent::EsFluentThis;

// In src/user/profile.rs
#[derive(EsFluentThis)]
#[fluent_this(origin)]
#[fluent(namespace = folder)]
pub enum FolderStatus {
    Active,
    Inactive,
}
```

A type in `src/user/profile.rs` maps to namespace `user`.

### Folder Relative

`namespace(folder(relative))` uses the parent folder path relative to the crate root, stripping `src/` when nested and keeping `src` for root module files.

```rust
use es_fluent::EsFluentThis;

// In src/user/profile.rs
#[derive(EsFluentThis)]
#[fluent_this(origin)]
#[fluent(namespace(folder(relative)))]
pub struct FolderUserProfile;
```

A type in `src/user/profile.rs` maps to namespace `user`.

## Quick Reference

| Syntax                        | Example source file | Resulting namespace |
| ----------------------------- | ------------------- | ------------------- |
| `namespace = "name"`          | any                 | `name`              |
| `namespace = file`            | `src/ui/button.rs`  | `button`            |
| `namespace(file(relative))`   | `src/ui/button.rs`  | `ui/button`         |
| `namespace = folder`          | `src/ui/button.rs`  | `ui`                |
| `namespace(folder(relative))` | `src/ui/button.rs`  | `ui`                |

## Validation

If `namespaces = [...]` is set in your `i18n.toml`, both the compiler (at compile-time) and the CLI will validate that explicit string-based namespaces used by your code match the provided allowlist. File-based and folder-based namespaces bypass validation since they're derived automatically from the source tree.
