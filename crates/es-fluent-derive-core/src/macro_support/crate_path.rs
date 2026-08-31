use super::*;

#[derive(Clone, Debug)]
pub struct ResolvedCratePath {
    tokens: TokenStream,
    rust_path: String,
}

impl ResolvedCratePath {
    pub fn resolve(package_name: &str, fallback_crate_ident: &str) -> Self {
        match crate_name(package_name) {
            // Rustdoc compiles a crate's examples in a synthetic crate while
            // retaining the documented package manifest. Resolve through the
            // passed `--extern` facade instead of that synthetic `crate` root.
            Ok(FoundCrate::Itself) if std::env::var_os("UNSTABLE_RUSTDOC_TEST_PATH").is_some() => {
                Self::fallback(fallback_crate_ident)
            },
            Ok(FoundCrate::Itself) => Self {
                tokens: quote! { crate },
                rust_path: "crate".to_string(),
            },
            Ok(FoundCrate::Name(name)) => {
                let ident = format_ident!("{name}");
                Self {
                    tokens: quote! { ::#ident },
                    rust_path: format!("::{name}"),
                }
            },
            Err(_) => Self::fallback(fallback_crate_ident),
        }
    }

    pub fn resolve_with_self_alias(package_name: &str, self_crate_ident: &str) -> Self {
        match crate_name(package_name) {
            Ok(FoundCrate::Itself) => Self::fallback(self_crate_ident),
            Ok(FoundCrate::Name(name)) => {
                let ident = format_ident!("{name}");
                Self {
                    tokens: quote! { ::#ident },
                    rust_path: format!("::{name}"),
                }
            },
            Err(_) => Self::fallback(self_crate_ident),
        }
    }

    pub fn fallback(crate_ident: &str) -> Self {
        let ident = format_ident!("{crate_ident}");
        Self {
            tokens: quote! { ::#ident },
            rust_path: format!("::{crate_ident}"),
        }
    }

    pub fn tokens(&self) -> &TokenStream {
        &self.tokens
    }

    pub fn rust_path(&self) -> &str {
        &self.rust_path
    }
}

pub fn resolve_crate_path(package_name: &str, fallback_crate_ident: &str) -> TokenStream {
    ResolvedCratePath::resolve(package_name, fallback_crate_ident)
        .tokens()
        .clone()
}

pub fn resolve_crate_path_with_self_alias(
    package_name: &str,
    self_crate_ident: &str,
) -> TokenStream {
    ResolvedCratePath::resolve_with_self_alias(package_name, self_crate_ident)
        .tokens()
        .clone()
}
