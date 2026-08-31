mod cargo;
mod cfg;
mod source;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::FallbackValidationDerive;
use cargo::cargo_source_roots;
pub(super) use cfg::{attributes_enable_test_only_derive, attributes_require_test};
#[cfg(test)]
pub(super) use source::literal_include_path;
pub(super) use source::{SourceDeclaration, collect_source_evidence};
use source::{canonical_path, mark_source_declaration, source_range};

pub(super) fn derive_requires_test(
    input: &syn::DeriveInput,
    derive: Option<FallbackValidationDerive>,
) -> bool {
    if attributes_require_test(&input.attrs)
        || attributes_enable_test_only_derive(&input.attrs, derive)
    {
        return true;
    }

    // Rustc removes active `cfg` and `cfg_attr` attributes before invoking a
    // derive. Follow only Cargo target roots and module branches that can own
    // this source file, then match the declaration by its stable source
    // location. Unresolved and macro-generated evidence remains strict.
    let Some(source_path) = input.ident.span().local_file() else {
        return false;
    };
    let source_path = canonical_path(&source_path);
    let Ok(source) = std::fs::read_to_string(&source_path) else {
        return false;
    };
    let source_text = input.ident.span().source_text();
    let Some(range) = source_range(&source, input.ident.span().start(), source_text.as_deref())
    else {
        return false;
    };
    let Some((marked_source, marker_ident)) =
        mark_source_declaration(&source, range, source_text.as_deref())
    else {
        return false;
    };
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_default();
    let target = SourceDeclaration {
        path: source_path.clone(),
        marked_source,
        marker_ident,
    };
    let mut evidence = Vec::new();
    let mut visited = HashSet::new();
    for root in cargo_source_roots(&manifest_dir) {
        let module_dir = root.path.parent().unwrap_or(Path::new(""));
        collect_source_evidence(
            &root.path,
            module_dir,
            root.test_only,
            &target,
            derive,
            &mut visited,
            &mut evidence,
        );
    }

    if evidence.is_empty() {
        let module_dir = source_path.parent().unwrap_or(Path::new(""));
        collect_source_evidence(
            &source_path,
            module_dir,
            false,
            &target,
            derive,
            &mut visited,
            &mut evidence,
        );
    }

    !evidence.is_empty() && evidence.into_iter().all(std::convert::identity)
}
