//! Shared namespace rules used by derive parsing and runtime registration.
use darling::FromMeta;
use std::{
    borrow::Cow,
    path::{Component, Path},
};

/// Namespace selection rules for FTL file output.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum NamespaceRule {
    /// A literal namespace string.
    Literal(Cow<'static, str>),
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
            Self::File => crate::namespace_resolver::file_stem_namespace(file_path),
            Self::FileRelative => {
                crate::namespace_resolver::file_relative_namespace(file_path, manifest_dir)
            },
            Self::Folder => crate::namespace_resolver::folder_namespace(file_path),
            Self::FolderRelative => {
                crate::namespace_resolver::folder_relative_namespace(file_path, manifest_dir)
            },
        }
    }
}

/// Validate a resolved namespace before using it as a relative output path.
pub fn validate_namespace_path(namespace: &str) -> Result<(), &'static str> {
    let trimmed = namespace.trim();
    if trimmed.is_empty() {
        return Err("namespace must not be empty");
    }
    if namespace != trimmed {
        return Err("namespace must not have leading or trailing whitespace");
    }
    if trimmed.contains('\\') {
        return Err("namespace must use '/' as path separator");
    }
    if trimmed.split('/').any(|segment| segment.is_empty()) {
        return Err("namespace path must not contain empty segments");
    }
    if trimmed
        .split('/')
        .any(|segment| matches!(segment, "." | ".."))
    {
        return Err("namespace path must not contain '.' or '..' segments");
    }
    if Path::new(trimmed)
        .components()
        .any(|component| matches!(component, Component::RootDir | Component::Prefix(_)))
    {
        return Err("namespace must be a relative path");
    }
    if trimmed.ends_with(".ftl") {
        return Err("namespace must not include file extension");
    }

    Ok(())
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
                    Ok(Self::Literal(Cow::Owned(s.value())))
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
                    }) => Ok(Self::Literal(Cow::Owned(lit.value()))),
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

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn literal_namespace_parses_and_resolves() {
        let meta: syn::Meta = parse_quote!(namespace = "my_namespace");
        let ns = NamespaceRule::from_meta(&meta).unwrap();
        assert!(matches!(ns, NamespaceRule::Literal(ref s) if s == "my_namespace"));
        assert_eq!(ns.resolve("/some/path/lib.rs", None), "my_namespace");
    }

    #[test]
    fn literal_namespace_constructor_accepts_static_str() {
        let ns = NamespaceRule::Literal(Cow::Borrowed("ui"));
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

    #[test]
    fn relative_namespace_resolution_normalizes_parent_segments() {
        assert_eq!(
            NamespaceRule::FileRelative.resolve("src/ui/../button.rs", None),
            "button"
        );
        assert_eq!(
            NamespaceRule::FolderRelative.resolve("src/ui/../forms/button.rs", None),
            "forms"
        );
    }

    #[test]
    fn namespace_rule_parses_list_and_name_value_relative_forms() {
        let file_relative_meta: syn::Meta = parse_quote!(namespace = file(relative));
        assert!(matches!(
            NamespaceRule::from_meta(&file_relative_meta).unwrap(),
            NamespaceRule::FileRelative
        ));

        let folder_meta: syn::Meta = parse_quote!(namespace(folder));
        assert!(matches!(
            NamespaceRule::from_meta(&folder_meta).unwrap(),
            NamespaceRule::Folder
        ));

        let literal_meta: syn::Meta = parse_quote!(namespace("ui/list"));
        assert!(matches!(
            NamespaceRule::from_meta(&literal_meta).unwrap(),
            NamespaceRule::Literal(ref value) if value == "ui/list"
        ));
    }

    #[test]
    fn validate_namespace_path_rejects_unsafe_values() {
        assert!(validate_namespace_path("ui/button").is_ok());
        assert_eq!(
            validate_namespace_path("").unwrap_err(),
            "namespace must not be empty"
        );
        assert_eq!(
            validate_namespace_path(" ui/button ").unwrap_err(),
            "namespace must not have leading or trailing whitespace"
        );
        assert_eq!(
            validate_namespace_path(r"ui\button").unwrap_err(),
            "namespace must use '/' as path separator"
        );
        assert_eq!(
            validate_namespace_path("ui//button").unwrap_err(),
            "namespace path must not contain empty segments"
        );
        assert_eq!(
            validate_namespace_path("../escape").unwrap_err(),
            "namespace path must not contain '.' or '..' segments"
        );
        assert_eq!(
            validate_namespace_path("/escape").unwrap_err(),
            "namespace path must not contain empty segments"
        );
        assert_eq!(
            validate_namespace_path("ui/button.ftl").unwrap_err(),
            "namespace must not include file extension"
        );
    }

    #[test]
    fn namespace_rule_rejects_unsupported_meta_shapes() {
        let unsupported_format: syn::Meta = parse_quote!(namespace);
        assert!(NamespaceRule::from_meta(&unsupported_format).is_err());

        let unknown_name_value_path: syn::Meta = parse_quote!(namespace = module);
        assert!(NamespaceRule::from_meta(&unknown_name_value_path).is_err());

        let unsupported_name_value_literal: syn::Meta = parse_quote!(namespace = 42);
        assert!(NamespaceRule::from_meta(&unsupported_name_value_literal).is_err());

        let unknown_list_path: syn::Meta = parse_quote!(namespace(module));
        assert!(NamespaceRule::from_meta(&unknown_list_path).is_err());

        let unsupported_list_literal: syn::Meta = parse_quote!(namespace(42));
        assert!(NamespaceRule::from_meta(&unsupported_list_literal).is_err());
    }

    #[test]
    fn relative_namespace_calls_require_supported_single_ident_arguments() {
        let unknown_target: syn::Meta = parse_quote!(namespace(module(relative)));
        assert!(NamespaceRule::from_meta(&unknown_target).is_err());

        let unknown_argument: syn::Meta = parse_quote!(namespace(file(crate_root)));
        assert!(NamespaceRule::from_meta(&unknown_argument).is_err());

        let multiple_arguments: syn::Meta = parse_quote!(namespace(file(relative, extra)));
        assert!(NamespaceRule::from_meta(&multiple_arguments).is_err());

        let literal_argument: syn::Meta = parse_quote!(namespace(file("relative")));
        assert!(NamespaceRule::from_meta(&literal_argument).is_err());
    }
}
