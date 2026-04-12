use std::path::{Component, Path, PathBuf};

pub(crate) fn file_stem_namespace(file_path: &str) -> String {
    Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

pub(crate) fn file_relative_namespace(file_path: &str, manifest_dir: Option<&Path>) -> String {
    let path = Path::new(file_path);
    let mut relative = path;

    if let Some(manifest_dir) = manifest_dir
        && path.is_absolute()
    {
        match path.strip_prefix(manifest_dir) {
            Ok(stripped) => relative = stripped,
            Err(_) => return file_stem_namespace(file_path),
        }
    }

    let mut without_ext = relative.to_path_buf();
    without_ext.set_extension("");

    let namespace_path = strip_src_prefix_if_not_empty(&without_ext);
    path_to_namespace(&namespace_path)
}

pub(crate) fn folder_namespace(file_path: &str) -> String {
    Path::new(file_path)
        .parent()
        .and_then(Path::file_name)
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

pub(crate) fn folder_relative_namespace(file_path: &str, manifest_dir: Option<&Path>) -> String {
    let path = Path::new(file_path);
    let mut relative = path;

    if let Some(manifest_dir) = manifest_dir
        && path.is_absolute()
    {
        match path.strip_prefix(manifest_dir) {
            Ok(stripped) => relative = stripped,
            Err(_) => return folder_namespace(file_path),
        }
    }

    let parent = relative.parent().unwrap_or(relative);
    let namespace_path = strip_src_prefix_if_not_empty(parent);
    path_to_namespace(&namespace_path)
}

fn strip_src_prefix_if_not_empty(path: &Path) -> PathBuf {
    if let Ok(stripped) = path.strip_prefix("src")
        && stripped.components().next().is_some()
    {
        return stripped.to_path_buf();
    }

    path.to_path_buf()
}

fn path_to_namespace(path: &Path) -> String {
    let mut parts: Vec<String> = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(segment) => {
                parts.push(segment.to_string_lossy().into_owned());
            },
            Component::ParentDir => parts.push("..".to_string()),
            Component::CurDir => {},
            Component::RootDir | Component::Prefix(_) => {},
        }
    }

    if parts.is_empty() {
        "unknown".to_string()
    } else {
        parts.join("/")
    }
}
