use proc_macro2::{Span, TokenStream};
use quote::quote;
use std::path::{Component, Path, PathBuf};

/// Returns tokens for the source file recorded in generated registry metadata.
///
/// The path is relative to `CARGO_MANIFEST_DIR` and uses `/` separators, so
/// path-derived namespaces and source links do not depend on the path rustc
/// reports through `file!()`. Compiler wrappers that pass
/// `--remap-path-prefix`, such as kache, otherwise make `file!()` resolve
/// outside the manifest directory. Falls back to `file!()` when the on-disk
/// path is unavailable or outside the manifest directory.
pub fn registry_source_file_tokens(span: Span) -> TokenStream {
    match manifest_relative_source_file(span) {
        Some(path) => quote! { #path },
        None => quote! { file!() },
    }
}

fn manifest_relative_source_file(span: Span) -> Option<String> {
    let source = span.local_file()?;
    let manifest_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR")?);
    let cwd = std::env::current_dir().ok()?;
    manifest_relative_path(&source, &cwd, &manifest_dir)
}

pub(super) fn manifest_relative_path(
    source: &Path,
    cwd: &Path,
    manifest_dir: &Path,
) -> Option<String> {
    let source = cwd.join(source);
    let relative = match source.strip_prefix(manifest_dir) {
        Ok(relative) => relative.to_path_buf(),
        Err(_) => {
            let source = std::fs::canonicalize(&source).ok()?;
            let manifest_dir = std::fs::canonicalize(manifest_dir).ok()?;
            source.strip_prefix(manifest_dir).ok()?.to_path_buf()
        },
    };

    let parts = relative
        .components()
        .map(|component| match component {
            Component::Normal(part) => part.to_str(),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    (!parts.is_empty()).then(|| parts.join("/"))
}
