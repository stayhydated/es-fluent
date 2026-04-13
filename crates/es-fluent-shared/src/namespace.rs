//! Shared namespace rules used by derive parsing and runtime registration.

use crate::namespace_resolver::{
    file_relative_namespace, file_stem_namespace, folder_namespace, folder_relative_namespace,
};
use darling::FromMeta;
use std::path::Path;

/// Namespace selection rules for FTL file output.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum NamespaceRule {
    /// A literal namespace string.
    Literal(&'static str),
    /// Use the source file name (stem only) as the namespace.
    File,
    /// Use the file path relative to the crate root as the namespace.
    FileRelative,
    /// Use the source file parent folder name as the namespace.
    Folder,
    /// Use the source file parent folder path relative to crate root as the namespace.
    FolderRelative,
}

impl NamespaceRule {
    /// Resolve the namespace string using the given file path.
    pub fn resolve(&self, file_path: &str, manifest_dir: Option<&Path>) -> String {
        match self {
            Self::Literal(value) => value.to_string(),
            Self::File => file_stem_namespace(file_path),
            Self::FileRelative => file_relative_namespace(file_path, manifest_dir),
            Self::Folder => folder_namespace(file_path),
            Self::FolderRelative => folder_relative_namespace(file_path, manifest_dir),
        }
    }
}

impl FromMeta for NamespaceRule {
    fn from_meta(item: &syn::Meta) -> darling::Result<Self> {
        match item {
            syn::Meta::NameValue(nv) => {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = &nv.value
                {
                    Ok(Self::Literal(leak_namespace_literal(s.value())))
                } else if let syn::Expr::Path(path) = &nv.value {
                    if path.path.is_ident("file") {
                        Ok(Self::File)
                    } else if path.path.is_ident("folder") {
                        Ok(Self::Folder)
                    } else {
                        Err(darling::Error::custom(
                            "expected string literal, 'file', or 'folder' identifier",
                        ))
                    }
                } else if let syn::Expr::Call(call) = &nv.value {
                    parse_relative_namespace(call)
                } else {
                    Err(darling::Error::unexpected_type(
                        "expected string literal, 'file', or 'folder'",
                    ))
                }
            },
            syn::Meta::List(list) => {
                let expr: syn::Expr = syn::parse2(list.tokens.clone()).map_err(|_| {
                    darling::Error::custom(
                        "expected string literal, 'file', 'folder', 'file(relative)', or 'folder(relative)'",
                    )
                })?;

                match expr {
                    syn::Expr::Path(path) => {
                        if path.path.is_ident("file") {
                            Ok(Self::File)
                        } else if path.path.is_ident("folder") {
                            Ok(Self::Folder)
                        } else {
                            Err(darling::Error::custom(
                                "expected string literal, 'file', 'folder', 'file(relative)', or 'folder(relative)'",
                            ))
                        }
                    },
                    syn::Expr::Call(call) => parse_relative_namespace(&call),
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(lit),
                        ..
                    }) => Ok(Self::Literal(leak_namespace_literal(lit.value()))),
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

fn parse_relative_namespace(call: &syn::ExprCall) -> darling::Result<NamespaceRule> {
    let Some((target, arg)) = parse_single_ident_call(call) else {
        return Err(darling::Error::custom(
            "expected string literal, 'file', 'folder', 'file(relative)', or 'folder(relative)'",
        ));
    };

    match (target.as_str(), arg.as_str()) {
        ("file", "relative") => Ok(NamespaceRule::FileRelative),
        ("folder", "relative") => Ok(NamespaceRule::FolderRelative),
        _ => Err(darling::Error::custom(
            "expected string literal, 'file', 'folder', 'file(relative)', or 'folder(relative)'",
        )),
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

fn leak_namespace_literal(value: String) -> &'static str {
    Box::leak(value.into_boxed_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn literal_namespace_parses_and_resolves() {
        let meta: syn::Meta = parse_quote!(namespace = "my_namespace");
        let ns = NamespaceRule::from_meta(&meta).unwrap();
        assert!(matches!(ns, NamespaceRule::Literal(ref s) if *s == "my_namespace"));
        assert_eq!(ns.resolve("/some/path/lib.rs", None), "my_namespace");
    }

    #[test]
    fn literal_namespace_constructor_accepts_static_str() {
        let ns = NamespaceRule::Literal("ui");
        assert_eq!(ns.resolve("/some/path/lib.rs", None), "ui");
    }

    #[test]
    fn file_and_folder_variants_parse() {
        let file_meta: syn::Meta = parse_quote!(namespace = file);
        assert!(matches!(
            NamespaceRule::from_meta(&file_meta).unwrap(),
            NamespaceRule::File
        ));

        let folder_meta: syn::Meta = parse_quote!(namespace(folder(relative)));
        assert!(matches!(
            NamespaceRule::from_meta(&folder_meta).unwrap(),
            NamespaceRule::FolderRelative
        ));
    }

    #[test]
    fn namespace_rule_resolves_relative_variants() {
        assert_eq!(
            NamespaceRule::FileRelative.resolve("src/ui/button.rs", None),
            "ui/button"
        );
        assert_eq!(
            NamespaceRule::FolderRelative.resolve("src/ui/button.rs", None),
            "ui"
        );
    }
}
