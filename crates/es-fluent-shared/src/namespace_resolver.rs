use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use std::path::Path;

pub(crate) fn file_stem_namespace(file_path: &str) -> String {
    Utf8Path::new(file_path)
        .file_stem()
        .map(str::to_owned)
        .unwrap_or_else(|| "unknown".to_string())
}

pub(crate) fn file_relative_namespace(file_path: &str, manifest_dir: Option<&Path>) -> String {
    let path = Utf8Path::new(file_path);
    let relative = match strip_manifest_prefix(path, manifest_dir) {
        Some(relative) => relative,
        None => return file_stem_namespace(file_path),
    };
    let without_ext = relative.with_extension("");

    let namespace_path = strip_src_prefix_if_not_empty(&without_ext);
    path_to_namespace(&namespace_path)
}

pub(crate) fn folder_namespace(file_path: &str) -> String {
    Utf8Path::new(file_path)
        .parent()
        .and_then(Utf8Path::file_name)
        .map(str::to_owned)
        .unwrap_or_else(|| "unknown".to_string())
}

pub(crate) fn folder_relative_namespace(file_path: &str, manifest_dir: Option<&Path>) -> String {
    let path = Utf8Path::new(file_path);
    let relative = match strip_manifest_prefix(path, manifest_dir) {
        Some(relative) => relative,
        None => return folder_namespace(file_path),
    };
    let parent = relative.parent().unwrap_or(relative);
    let namespace_path = strip_src_prefix_if_not_empty(parent);
    path_to_namespace(&namespace_path)
}

fn strip_manifest_prefix<'a>(
    path: &'a Utf8Path,
    manifest_dir: Option<&Path>,
) -> Option<&'a Utf8Path> {
    if let Some(manifest_dir) = manifest_dir
        && path.is_absolute()
    {
        let manifest_dir = Utf8Path::from_path(manifest_dir)?;
        return path.strip_prefix(manifest_dir).ok();
    }

    Some(path)
}

fn strip_src_prefix_if_not_empty(path: &Utf8Path) -> Utf8PathBuf {
    if let Ok(stripped) = path.strip_prefix("src")
        && stripped.components().next().is_some()
    {
        return stripped.to_path_buf();
    }

    path.to_path_buf()
}

fn path_to_namespace(path: &Utf8Path) -> String {
    let mut parts: Vec<String> = Vec::new();
    for component in path.components() {
        match component {
            Utf8Component::Normal(segment) => {
                parts.push(segment.to_string());
            },
            Utf8Component::ParentDir => {
                if parts.last().is_some_and(|last| last != "..") {
                    parts.pop();
                } else {
                    parts.push("..".to_string());
                }
            },
            Utf8Component::CurDir => {},
            Utf8Component::RootDir | Utf8Component::Prefix(_) => {},
        }
    }

    if parts.is_empty() {
        "unknown".to_string()
    } else {
        parts.join("/")
    }
}
