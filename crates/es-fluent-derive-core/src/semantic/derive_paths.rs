use crate::error::{AttrContext, AttrError, ErrorExt as _, EsFluentCoreError, EsFluentCoreResult};
use proc_macro2::Span;
use quote::ToTokens as _;
use syn::spanned::Spanned as _;

/// A validated derive path for a generated enum.
#[derive(Clone, Debug)]
pub struct DerivePath {
    path: syn::Path,
    span: Span,
}

impl DerivePath {
    pub fn new(path: syn::Path, context: AttrContext) -> EsFluentCoreResult<Self> {
        let span = path.span();
        if path.segments.is_empty() {
            return Err(EsFluentCoreError::StructuredAttributeError(AttrError::new(
                context,
                "derive path must not be empty",
                Some(span),
            )));
        }

        Ok(Self { path, span })
    }

    pub fn path(&self) -> &syn::Path {
        &self.path
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn to_token_string(&self) -> String {
        self.path.to_token_stream().to_string()
    }
}

/// Validated derive paths for a generated enum.
#[derive(Clone, Debug, Default)]
pub struct DerivePathList {
    paths: Vec<DerivePath>,
}

impl DerivePathList {
    pub fn from_paths(
        paths: impl IntoIterator<Item = syn::Path>,
        context: AttrContext,
    ) -> EsFluentCoreResult<Self> {
        let mut seen = std::collections::HashSet::new();
        let mut derive_paths = Vec::new();
        for path in paths {
            let path = DerivePath::new(path, context)?;
            if seen.insert(derive_path_dedup_key(path.path())) {
                derive_paths.push(path);
            }
        }

        Ok(Self {
            paths: derive_paths,
        })
    }

    pub fn for_generated_variants(
        paths: impl IntoIterator<Item = syn::Path>,
        context: AttrContext,
    ) -> EsFluentCoreResult<Self> {
        let defaults: Vec<syn::Path> = vec![
            syn::parse_quote!(Clone),
            syn::parse_quote!(Copy),
            syn::parse_quote!(Debug),
            syn::parse_quote!(Eq),
            syn::parse_quote!(Hash),
            syn::parse_quote!(PartialEq),
        ];

        let paths = paths.into_iter().collect::<Vec<_>>();
        for path in &paths {
            if is_es_fluent_choice_derive_path(path) {
                return Err(EsFluentCoreError::StructuredAttributeError(AttrError::new(
                    context,
                    "generated variant enums implement EsFluentChoice automatically",
                    Some(path.span()),
                ))
                .with_help(
                    "remove EsFluentChoice from #[fluent_variants(derive(...))]".to_string(),
                ));
            }
        }

        Self::from_paths(defaults.into_iter().chain(paths), context)
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    pub fn paths(&self) -> &[DerivePath] {
        &self.paths
    }

    pub fn token_strings(&self) -> Vec<String> {
        self.paths.iter().map(DerivePath::to_token_string).collect()
    }
}

fn derive_path_dedup_key(path: &syn::Path) -> String {
    let Some(last_segment) = path.segments.last() else {
        return path.to_token_stream().to_string();
    };

    let ident = last_segment.ident.to_string();
    if matches!(
        ident.as_str(),
        "Clone" | "Copy" | "Debug" | "Eq" | "Hash" | "PartialEq"
    ) {
        ident
    } else {
        path.to_token_stream().to_string()
    }
}

fn is_es_fluent_choice_derive_path(path: &syn::Path) -> bool {
    path.segments
        .last()
        .is_some_and(|segment| segment.ident == "EsFluentChoice")
}
