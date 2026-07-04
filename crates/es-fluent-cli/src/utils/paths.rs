use path_slash::PathExt as _;
use std::path::{Path, PathBuf};

pub(crate) fn slash_path(path: &Path) -> String {
    normalize_windows_verbatim_path(path)
        .to_slash_lossy()
        .into_owned()
}

pub(crate) fn relative_slash_path(path: &Path, base: &Path) -> String {
    let path = normalize_windows_verbatim_path(path);
    let base = normalize_windows_verbatim_path(base);
    let path_canon = std::fs::canonicalize(&path)
        .map(|path| normalize_windows_verbatim_path(&path))
        .unwrap_or_else(|_| path.clone());
    let base_canon = std::fs::canonicalize(&base)
        .map(|path| normalize_windows_verbatim_path(&path))
        .unwrap_or_else(|_| base.clone());

    if let Ok(rel) = path_canon.strip_prefix(&base_canon) {
        return slash_path(rel);
    }

    if let Ok(rel) = path.strip_prefix(&base) {
        return slash_path(rel);
    }

    slash_path(&path)
}

pub(crate) fn relative_slash_message(message: &str, base: &Path) -> String {
    let raw_base = base.to_path_buf();
    let base = normalize_windows_verbatim_path(base);
    let raw_base_canon = std::fs::canonicalize(&raw_base).unwrap_or_else(|_| raw_base.clone());
    let base_canon = normalize_windows_verbatim_path(&raw_base_canon);

    let mut normalized = message.to_string();
    for prefix in [&raw_base_canon, &base_canon, &raw_base, &base] {
        normalized = replace_path_prefix(&normalized, prefix);
    }
    normalized.replace('\\', "/")
}

pub(crate) fn normalize_windows_verbatim_path(path: &Path) -> PathBuf {
    normalize_windows_verbatim_path_inner(path)
}

#[cfg(windows)]
fn normalize_windows_verbatim_path_inner(path: &Path) -> PathBuf {
    use std::path::{Component, Prefix};

    let mut components = path.components();
    let Some(Component::Prefix(prefix)) = components.next() else {
        return path.to_path_buf();
    };

    let (mut normalized, skip_root) = match prefix.kind() {
        Prefix::VerbatimDisk(disk) => (PathBuf::from(format!("{}:\\", disk as char)), true),
        Prefix::VerbatimUNC(server, share) => (
            PathBuf::from(format!(
                r"\\{}\{}",
                server.to_string_lossy(),
                share.to_string_lossy()
            )),
            true,
        ),
        Prefix::Verbatim(prefix) => (PathBuf::from(prefix), false),
        _ => return path.to_path_buf(),
    };

    for component in components {
        if skip_root && matches!(component, Component::RootDir) {
            continue;
        }
        normalized.push(component.as_os_str());
    }

    normalized
}

#[cfg(not(windows))]
fn normalize_windows_verbatim_path_inner(path: &Path) -> PathBuf {
    path.to_path_buf()
}

fn replace_path_prefix(message: &str, base: &Path) -> String {
    let native = base.display().to_string();
    let slash = base.to_slash_lossy().into_owned();
    let mut normalized = message.to_string();

    for base in [native.as_str(), slash.as_str()] {
        if base.is_empty() {
            continue;
        }

        normalized = normalized
            .replace(&format!("{base}/"), "")
            .replace(&format!("{base}\\"), "");
    }

    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_slash_path_uses_forward_slashes() {
        let temp = tempfile::tempdir().expect("tempdir");
        let ftl = temp.path().join("i18n/en/test-app.ftl");
        std::fs::create_dir_all(ftl.parent().expect("parent")).expect("create parent");
        std::fs::write(&ftl, "hello = Hello\n").expect("write ftl");

        assert_eq!(
            relative_slash_path(&ftl, temp.path()),
            "i18n/en/test-app.ftl"
        );
    }

    #[test]
    fn relative_slash_message_strips_workspace_prefixes() {
        let temp = tempfile::tempdir().expect("tempdir");
        let message = format!(
            "failed at {}",
            temp.path().join("i18n/en/test-app.ftl").display()
        );

        assert_eq!(
            relative_slash_message(&message, temp.path()),
            "failed at i18n/en/test-app.ftl"
        );
    }
}
