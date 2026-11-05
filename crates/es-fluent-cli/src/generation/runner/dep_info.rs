//! Runner staleness evidence from the dep-info file Cargo writes beside the binary.
//!
//! The content hashes in the runner cache cover the selected i18n crates and the
//! workspace Cargo inputs. Cargo's dep-info additionally lists every local source
//! file the runner was compiled from, including path dependencies, `[patch]`
//! paths, `include_str!` inputs, and build-script `rerun-if-changed` paths.

use fs_err as fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Returns the dep-info path Cargo writes for the runner binary at `binary_path`.
pub(super) fn dep_info_path(binary_path: &Path) -> PathBuf {
    let stem = binary_path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    binary_path.with_file_name(format!("{stem}.d"))
}

/// Reports whether any input Cargo recorded for the runner changed after `built_at`.
///
/// A missing or relative-path dep-info file provides no evidence and returns
/// `false`, leaving staleness to the content hashes. A listed input that is
/// missing counts as changed when its nearest existing ancestor directory
/// changed after `built_at`, which removing it does. Paths that never existed,
/// such as a build script's `rerun-if-changed` path for a directory not created
/// yet, do not keep the runner stale.
pub(super) fn inputs_changed_since(dep_info: &Path, built_at: SystemTime) -> bool {
    let Ok(contents) = fs::read_to_string(dep_info) else {
        return false;
    };
    let inputs = parse_dep_info(&contents);
    if inputs.iter().any(|path| path.is_relative()) {
        return false;
    }

    inputs.iter().any(|path| match modified(path) {
        Some(modified) => modified > built_at,
        None => path
            .ancestors()
            .skip(1)
            .find_map(modified)
            .is_none_or(|modified| modified > built_at),
    })
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
}

/// Parses the prerequisites of a Makefile-style dep-info file written by Cargo.
///
/// Cargo writes one `target: input input ...` rule per line and escapes spaces
/// inside paths as `\ `.
pub(super) fn parse_dep_info(contents: &str) -> Vec<PathBuf> {
    let mut inputs = Vec::new();
    for line in contents.lines() {
        let Some((_, prerequisites)) = line.split_once(": ") else {
            continue;
        };

        let mut current = String::new();
        let mut chars = prerequisites.chars().peekable();
        while let Some(ch) = chars.next() {
            match ch {
                '\\' if chars.peek() == Some(&' ') => {
                    current.push(' ');
                    chars.next();
                },
                ' ' | '\t' => {
                    if !current.is_empty() {
                        inputs.push(PathBuf::from(std::mem::take(&mut current)));
                    }
                },
                _ => current.push(ch),
            }
        }
        if !current.is_empty() {
            inputs.push(PathBuf::from(current));
        }
    }
    inputs
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn dep_info_path_sits_beside_binary_without_extension() {
        assert_eq!(
            dep_info_path(Path::new("/w/.es-fluent/target/debug/es-fluent-runner")),
            PathBuf::from("/w/.es-fluent/target/debug/es-fluent-runner.d")
        );
        assert_eq!(
            dep_info_path(Path::new(r"C:\w\target\debug\es-fluent-runner.exe")),
            PathBuf::from(r"C:\w\target\debug\es-fluent-runner.d")
        );
    }

    #[test]
    fn parse_dep_info_reads_prerequisites_and_escaped_spaces() {
        let contents = "/w/target/debug/es-fluent-runner: /w/src/main.rs /w/my\\ crate/src/lib.rs\n\n/w/other: /w/build.rs\n";
        assert_eq!(
            parse_dep_info(contents),
            vec![
                PathBuf::from("/w/src/main.rs"),
                PathBuf::from("/w/my crate/src/lib.rs"),
                PathBuf::from("/w/build.rs"),
            ]
        );
    }

    #[test]
    fn parse_dep_info_keeps_windows_drive_prefixes() {
        let contents =
            r"C:\w\target\debug\es-fluent-runner.exe: C:\w\src\main.rs C:\w\crates\a\src\lib.rs";
        assert_eq!(
            parse_dep_info(contents),
            vec![
                PathBuf::from(r"C:\w\src\main.rs"),
                PathBuf::from(r"C:\w\crates\a\src\lib.rs"),
            ]
        );
    }

    #[test]
    fn inputs_changed_since_compares_input_mtimes_with_build_time() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("lib.rs");
        std::fs::write(&source, "pub struct Demo;\n").expect("write source");
        let dep_info = temp.path().join("runner.d");
        std::fs::write(
            &dep_info,
            format!(
                "{}: {}\n",
                temp.path().join("runner").display(),
                source.display()
            ),
        )
        .expect("write dep-info");

        let source_mtime = std::fs::metadata(&source)
            .and_then(|metadata| metadata.modified())
            .expect("source mtime");

        assert!(!inputs_changed_since(
            &dep_info,
            source_mtime + Duration::from_secs(1)
        ));
        assert!(inputs_changed_since(
            &dep_info,
            source_mtime - Duration::from_secs(1)
        ));

        std::fs::remove_file(&source).expect("remove source");
        let removed_at = std::fs::metadata(temp.path())
            .and_then(|metadata| metadata.modified())
            .expect("parent mtime");
        assert!(
            inputs_changed_since(&dep_info, removed_at - Duration::from_secs(1)),
            "an input removed after the build must count as changed"
        );
    }

    #[test]
    fn inputs_that_never_existed_do_not_keep_the_runner_stale() {
        let temp = tempfile::tempdir().expect("tempdir");
        let parent = temp.path().join("package");
        std::fs::create_dir(&parent).expect("create package dir");
        let dep_info = temp.path().join("runner.d");
        std::fs::write(
            &dep_info,
            format!(
                "{}: {}\n",
                temp.path().join("runner").display(),
                parent.join("i18n/missing").display()
            ),
        )
        .expect("write dep-info");
        let parent_mtime = std::fs::metadata(&parent)
            .and_then(|metadata| metadata.modified())
            .expect("parent mtime");

        assert!(!inputs_changed_since(
            &dep_info,
            parent_mtime + Duration::from_secs(1)
        ));
        assert!(
            inputs_changed_since(&dep_info, parent_mtime - Duration::from_secs(1)),
            "a parent changed after the build may have lost the input"
        );
    }

    #[test]
    fn inputs_changed_since_ignores_unusable_dep_info() {
        let temp = tempfile::tempdir().expect("tempdir");
        let built_at = SystemTime::now();

        assert!(!inputs_changed_since(
            &temp.path().join("missing.d"),
            built_at
        ));

        let relative = temp.path().join("relative.d");
        std::fs::write(&relative, "runner: src/main.rs\n").expect("write dep-info");
        assert!(!inputs_changed_since(&relative, built_at));
    }
}
