//! Namespace attribute for FTL file generation.

use darling::FromMeta;

/// Represents the namespace attribute value.
///
/// Supports:
/// - `namespace = "some_name"` - literal namespace
/// - `namespace = file` - use the source file name (e.g., `lib.rs` -> `lib`)
/// - `namespace = file(relative)` - use file path relative to crate root (e.g., `src/ui/button.rs` -> `src/ui/button`)
#[derive(Clone, Debug)]
pub enum NamespaceValue {
    /// A literal namespace string.
    Literal(String),
    /// Use the source file name (stem only) as the namespace.
    File,
    /// Use the file path relative to the crate root as the namespace.
    FileRelative,
}

impl NamespaceValue {
    /// Returns the namespace string, resolving "file" variants to actual paths.
    ///
    /// For `FileRelative`, this strips the `src/` prefix if present and removes the extension.
    pub fn resolve(&self, file_path: &str) -> String {
        match self {
            NamespaceValue::Literal(s) => s.clone(),
            NamespaceValue::File => std::path::Path::new(file_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string(),
            NamespaceValue::FileRelative => {
                // Strip src/ prefix if present and remove extension
                let path = std::path::Path::new(file_path);
                let without_ext = path.with_extension("");
                let path_str = without_ext.to_str().unwrap_or("unknown");

                // Strip common prefixes like "src/"
                path_str
                    .strip_prefix("src/")
                    .or_else(|| path_str.strip_prefix("src\\"))
                    .unwrap_or(path_str)
                    .to_string()
            },
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
                    // namespace = file (without quotes - parsed as path)
                    if path.path.is_ident("file") {
                        return Ok(NamespaceValue::File);
                    }
                    Err(darling::Error::custom(
                        "expected string literal or 'file' identifier",
                    ))
                } else {
                    Err(darling::Error::unexpected_type(
                        "expected string literal or 'file'",
                    ))
                }
            },
            // namespace(file) or namespace(file(relative))
            syn::Meta::List(list) => {
                let tokens = list.tokens.to_string();
                // Normalize whitespace for comparison
                let normalized: String = tokens.split_whitespace().collect();

                // namespace(file(relative)) -> use relative path
                if normalized == "file(relative)" {
                    return Ok(NamespaceValue::FileRelative);
                }

                // namespace(file) -> use file name
                if normalized == "file" {
                    return Ok(NamespaceValue::File);
                }

                // Try to parse as a string literal
                let lit: syn::LitStr = syn::parse2(list.tokens.clone()).map_err(|_| {
                    darling::Error::custom("expected string literal, 'file', or 'file(relative)'")
                })?;
                Ok(NamespaceValue::Literal(lit.value()))
            },
            _ => Err(darling::Error::unsupported_format(
                "expected namespace = \"value\", namespace = file, or namespace = file(relative)",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_literal_namespace() {
        let meta: syn::Meta = parse_quote!(namespace = "my_namespace");
        let ns = NamespaceValue::from_meta(&meta).unwrap();
        match ns {
            NamespaceValue::Literal(s) => assert_eq!(s, "my_namespace"),
            _ => panic!("expected literal"),
        }
    }

    #[test]
    fn test_file_namespace() {
        let meta: syn::Meta = parse_quote!(namespace = file);
        let ns = NamespaceValue::from_meta(&meta).unwrap();
        match ns {
            NamespaceValue::File => {},
            _ => panic!("expected file"),
        }
    }

    #[test]
    fn test_file_relative_namespace() {
        let meta: syn::Meta = parse_quote!(namespace(file(relative)));
        let ns = NamespaceValue::from_meta(&meta).unwrap();
        match ns {
            NamespaceValue::FileRelative => {},
            _ => panic!("expected FileRelative"),
        }
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
}
