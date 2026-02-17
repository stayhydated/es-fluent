//! Namespace attribute for FTL file generation.

use crate::namespace_resolver::{
    file_relative_namespace, file_stem_namespace, folder_namespace, folder_relative_namespace,
};
use darling::FromMeta;

/// Represents the namespace attribute value.
///
/// Supports:
/// - `namespace = "some_name"` - literal namespace
/// - `namespace = file` - use the source file name (e.g., `lib.rs` -> `lib`)
/// - `namespace = file(relative)` - use file path relative to crate root (e.g., `src/ui/button.rs` -> `ui/button`)
/// - `namespace = folder` - use the source file parent folder (e.g., `src/ui/button.rs` -> `ui`)
/// - `namespace = folder(relative)` - use parent folder path relative to crate root (e.g., `src/ui/button.rs` -> `ui`)
#[derive(Clone, Debug)]
pub enum NamespaceValue {
    /// A literal namespace string.
    Literal(String),
    /// Use the source file name (stem only) as the namespace.
    File,
    /// Use the file path relative to the crate root as the namespace.
    FileRelative,
    /// Use the source file parent folder as the namespace.
    Folder,
    /// Use the source file parent folder path relative to crate root as the namespace.
    FolderRelative,
}

impl NamespaceValue {
    /// Returns the namespace string, resolving "file" variants to actual paths.
    ///
    /// For `FileRelative`, this strips the `src/` prefix if present and removes the extension.
    pub fn resolve(&self, file_path: &str) -> String {
        match self {
            NamespaceValue::Literal(s) => s.clone(),
            NamespaceValue::File => file_stem_namespace(file_path),
            NamespaceValue::FileRelative => file_relative_namespace(file_path, None),
            NamespaceValue::Folder => folder_namespace(file_path),
            NamespaceValue::FolderRelative => folder_relative_namespace(file_path, None),
        }
    }
}

impl FromMeta for NamespaceValue {
    fn from_meta(item: &syn::Meta) -> darling::Result<Self> {
        match item {
            // namespace = "literal_string"
            syn::Meta::NameValue(nv) => {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = &nv.value
                {
                    Ok(NamespaceValue::Literal(s.value()))
                } else if let syn::Expr::Path(path) = &nv.value {
                    // namespace = file | folder (without quotes - parsed as path)
                    if path.path.is_ident("file") {
                        return Ok(NamespaceValue::File);
                    }
                    if path.path.is_ident("folder") {
                        return Ok(NamespaceValue::Folder);
                    }
                    Err(darling::Error::custom(
                        "expected string literal, 'file', or 'folder' identifier",
                    ))
                } else if let syn::Expr::Call(call) = &nv.value {
                    let Some((target, arg)) = parse_single_ident_call(call) else {
                        return Err(darling::Error::custom(
                            "expected string literal, 'file', 'folder', or '(file|folder)(relative)'",
                        ));
                    };

                    if arg == "relative" {
                        if target == "file" {
                            return Ok(NamespaceValue::FileRelative);
                        }
                        if target == "folder" {
                            return Ok(NamespaceValue::FolderRelative);
                        }
                    }

                    Err(darling::Error::custom(
                        "expected string literal, 'file', 'folder', 'file(relative)', or 'folder(relative)'",
                    ))
                } else {
                    Err(darling::Error::unexpected_type(
                        "expected string literal, 'file', or 'folder'",
                    ))
                }
            },
            // namespace(file) / namespace(folder) / namespace(file(relative)) / namespace(folder(relative))
            syn::Meta::List(list) => {
                let expr: syn::Expr = syn::parse2(list.tokens.clone()).map_err(|_| {
                    darling::Error::custom(
                        "expected string literal, 'file', 'folder', 'file(relative)', or 'folder(relative)'",
                    )
                })?;

                match expr {
                    syn::Expr::Path(path) => {
                        if path.path.is_ident("file") {
                            return Ok(NamespaceValue::File);
                        }
                        if path.path.is_ident("folder") {
                            return Ok(NamespaceValue::Folder);
                        }
                        Err(darling::Error::custom(
                            "expected string literal, 'file', 'folder', 'file(relative)', or 'folder(relative)'",
                        ))
                    },
                    syn::Expr::Call(call) => {
                        let Some((target, arg)) = parse_single_ident_call(&call) else {
                            return Err(darling::Error::custom(
                                "expected string literal, 'file', 'folder', 'file(relative)', or 'folder(relative)'",
                            ));
                        };

                        if arg == "relative" {
                            if target == "file" {
                                return Ok(NamespaceValue::FileRelative);
                            }
                            if target == "folder" {
                                return Ok(NamespaceValue::FolderRelative);
                            }
                        }

                        Err(darling::Error::custom(
                            "expected string literal, 'file', 'folder', 'file(relative)', or 'folder(relative)'",
                        ))
                    },
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(lit),
                        ..
                    }) => Ok(NamespaceValue::Literal(lit.value())),
                    _ => Err(darling::Error::custom(
                        "expected string literal, 'file', 'folder', 'file(relative)', or 'folder(relative)'",
                    )),
                }
            },
            _ => Err(darling::Error::unsupported_format(
                "expected namespace = \"value\", namespace = file|folder, or namespace = file(relative)|folder(relative)",
            )),
        }
    }
}

fn parse_single_ident_call(call: &syn::ExprCall) -> Option<(String, String)> {
    let syn::Expr::Path(target_path) = call.func.as_ref() else {
        return None;
    };
    if call.args.len() != 1 {
        return None;
    }
    let arg = call.args.first()?;
    let syn::Expr::Path(arg_path) = arg else {
        return None;
    };
    let target = target_path.path.get_ident()?.to_string();
    let arg = arg_path.path.get_ident()?.to_string();
    Some((target, arg))
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_literal_namespace() {
        let meta: syn::Meta = parse_quote!(namespace = "my_namespace");
        let ns = NamespaceValue::from_meta(&meta).unwrap();
        assert!(matches!(ns, NamespaceValue::Literal(ref s) if s == "my_namespace"));
    }

    #[test]
    fn test_file_namespace() {
        let meta: syn::Meta = parse_quote!(namespace = file);
        let ns = NamespaceValue::from_meta(&meta).unwrap();
        assert!(matches!(ns, NamespaceValue::File));
    }

    #[test]
    fn test_file_relative_namespace() {
        let meta: syn::Meta = parse_quote!(namespace(file(relative)));
        let ns = NamespaceValue::from_meta(&meta).unwrap();
        assert!(matches!(ns, NamespaceValue::FileRelative));
    }

    #[test]
    fn test_folder_namespace() {
        let meta: syn::Meta = parse_quote!(namespace = folder);
        let ns = NamespaceValue::from_meta(&meta).unwrap();
        assert!(matches!(ns, NamespaceValue::Folder));
    }

    #[test]
    fn test_folder_relative_namespace() {
        let meta: syn::Meta = parse_quote!(namespace(folder(relative)));
        let ns = NamespaceValue::from_meta(&meta).unwrap();
        assert!(matches!(ns, NamespaceValue::FolderRelative));
    }

    #[test]
    fn test_folder_relative_namespace_name_value_call_syntax() {
        let meta: syn::Meta = parse_quote!(namespace = folder(relative));
        let ns = NamespaceValue::from_meta(&meta).unwrap();
        assert!(matches!(ns, NamespaceValue::FolderRelative));
    }

    #[test]
    fn test_file_relative_namespace_name_value_call_syntax() {
        let meta: syn::Meta = parse_quote!(namespace = file(relative));
        let ns = NamespaceValue::from_meta(&meta).unwrap();
        assert!(matches!(ns, NamespaceValue::FileRelative));
    }

    #[test]
    fn test_list_path_namespace_variants() {
        let file_meta: syn::Meta = parse_quote!(namespace(file));
        let file_ns = NamespaceValue::from_meta(&file_meta).unwrap();
        assert!(matches!(file_ns, NamespaceValue::File));

        let folder_meta: syn::Meta = parse_quote!(namespace(folder));
        let folder_ns = NamespaceValue::from_meta(&folder_meta).unwrap();
        assert!(matches!(folder_ns, NamespaceValue::Folder));
    }

    #[test]
    fn test_resolve_literal() {
        let ns = NamespaceValue::Literal("my_ns".to_string());
        assert_eq!(ns.resolve("/some/path/lib.rs"), "my_ns");
    }

    #[test]
    fn test_resolve_file() {
        let ns = NamespaceValue::File;
        assert_eq!(ns.resolve("/some/path/lib.rs"), "lib");
        assert_eq!(ns.resolve("src/components/button.rs"), "button");
    }

    #[test]
    fn test_resolve_file_relative() {
        let ns = NamespaceValue::FileRelative;
        assert_eq!(ns.resolve("src/ui/button.rs"), "ui/button");
        assert_eq!(ns.resolve("src/lib.rs"), "lib");
        assert_eq!(ns.resolve("lib.rs"), "lib");
    }

    #[test]
    fn test_resolve_folder() {
        let ns = NamespaceValue::Folder;
        assert_eq!(ns.resolve("src/ui/button.rs"), "ui");
        assert_eq!(ns.resolve("src/lib.rs"), "src");
        assert_eq!(ns.resolve("lib.rs"), "unknown");
    }

    #[test]
    fn test_resolve_folder_relative() {
        let ns = NamespaceValue::FolderRelative;
        assert_eq!(ns.resolve("src/ui/button.rs"), "ui");
        assert_eq!(ns.resolve("src/lib.rs"), "src");
        assert_eq!(ns.resolve("lib.rs"), "unknown");
    }

    #[test]
    fn test_name_value_and_list_error_paths() {
        let bad_ident: syn::Meta = parse_quote!(namespace = invalid_ident);
        let err = NamespaceValue::from_meta(&bad_ident).expect_err("invalid identifier");
        assert!(err.to_string().contains("expected string literal"));

        let bad_call_target: syn::Meta = parse_quote!(namespace = unknown(relative));
        let err = NamespaceValue::from_meta(&bad_call_target).expect_err("invalid call target");
        assert!(err.to_string().contains("file(relative)"));

        let bad_call_arg: syn::Meta = parse_quote!(namespace = file(absolute));
        let err = NamespaceValue::from_meta(&bad_call_arg).expect_err("invalid call argument");
        assert!(err.to_string().contains("folder(relative)"));

        let malformed_name_value_call: syn::Meta = parse_quote!(namespace = file(relative, extra));
        let err =
            NamespaceValue::from_meta(&malformed_name_value_call).expect_err("invalid call arity");
        assert!(err.to_string().contains("expected string literal"));

        let non_string: syn::Meta = parse_quote!(namespace = 123);
        let err = NamespaceValue::from_meta(&non_string).expect_err("non-string literal");
        assert!(!err.to_string().is_empty());

        let list_unknown: syn::Meta = parse_quote!(namespace(unknown));
        let err = NamespaceValue::from_meta(&list_unknown).expect_err("unknown list path");
        assert!(err.to_string().contains("file(relative)"));

        let list_non_expr_path: syn::Meta = parse_quote!(namespace(123));
        let err = NamespaceValue::from_meta(&list_non_expr_path).expect_err("unsupported expr");
        assert!(err.to_string().contains("file(relative)"));

        let malformed_list_call = syn::parse_str::<syn::Meta>("namespace(file, relative)")
            .expect("meta parse should succeed");
        let err =
            NamespaceValue::from_meta(&malformed_list_call).expect_err("list parse2 should fail");
        assert!(err.to_string().contains("file(relative)"));

        let malformed_list_nested_call: syn::Meta = parse_quote!(namespace((file)(relative)));
        let err = NamespaceValue::from_meta(&malformed_list_nested_call)
            .expect_err("nested call should be rejected");
        assert!(err.to_string().contains("file(relative)"));

        let list_literal: syn::Meta = parse_quote!(namespace("literal_ns"));
        let ns = NamespaceValue::from_meta(&list_literal).expect("list literal should work");
        assert!(matches!(ns, NamespaceValue::Literal(ref s) if s == "literal_ns"));

        let path_meta: syn::Meta = parse_quote!(namespace);
        let err = NamespaceValue::from_meta(&path_meta).expect_err("unsupported format");
        assert!(err.to_string().contains("expected namespace ="));
    }

    #[test]
    fn test_parse_single_ident_call_variants() {
        let valid: syn::ExprCall = parse_quote!(file(relative));
        assert_eq!(
            parse_single_ident_call(&valid),
            Some(("file".to_string(), "relative".to_string()))
        );

        let invalid_func: syn::ExprCall = parse_quote!((file)(relative));
        assert_eq!(parse_single_ident_call(&invalid_func), None);

        let invalid_arity: syn::ExprCall = parse_quote!(file(relative, extra));
        assert_eq!(parse_single_ident_call(&invalid_arity), None);

        let invalid_arg_kind: syn::ExprCall = parse_quote!(file("relative"));
        assert_eq!(parse_single_ident_call(&invalid_arg_kind), None);

        let invalid_target_ident: syn::ExprCall = parse_quote!(file::nested(relative));
        assert_eq!(parse_single_ident_call(&invalid_target_ident), None);

        let invalid_arg_ident: syn::ExprCall = parse_quote!(file(relative::nested));
        assert_eq!(parse_single_ident_call(&invalid_arg_ident), None);
    }
}
