use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::source::canonical_path;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(super) struct SourceRoot {
    pub(super) path: PathBuf,
    pub(super) test_only: bool,
}

pub(super) fn cargo_source_roots(manifest_dir: &Path) -> Vec<SourceRoot> {
    let manifest = std::fs::read_to_string(manifest_dir.join("Cargo.toml"))
        .ok()
        .and_then(|source| toml::from_str::<toml::Value>(&source).ok());
    let package = manifest
        .as_ref()
        .and_then(|manifest| manifest.get("package"))
        .and_then(toml::Value::as_table);
    let mut roots = Vec::new();

    if let Some(library) = manifest
        .as_ref()
        .and_then(|manifest| manifest.get("lib"))
        .and_then(toml::Value::as_table)
    {
        add_source_root(
            &mut roots,
            manifest_dir,
            library.get("path").and_then(toml::Value::as_str),
            "src/lib.rs",
            false,
        );
    } else if package_bool(package, "autolib") {
        add_existing_root(&mut roots, manifest_dir.join("src/lib.rs"), false);
    }

    add_declared_target_roots(
        &mut roots,
        manifest.as_ref(),
        manifest_dir,
        "bin",
        "src/bin",
        false,
    );
    add_declared_target_roots(
        &mut roots,
        manifest.as_ref(),
        manifest_dir,
        "test",
        "tests",
        true,
    );
    add_declared_target_roots(
        &mut roots,
        manifest.as_ref(),
        manifest_dir,
        "example",
        "examples",
        false,
    );
    add_declared_target_roots(
        &mut roots,
        manifest.as_ref(),
        manifest_dir,
        "bench",
        "benches",
        false,
    );

    if package_bool(package, "autobins") {
        add_existing_root(&mut roots, manifest_dir.join("src/main.rs"), false);
        add_auto_target_roots(&mut roots, &manifest_dir.join("src/bin"), false);
    }
    if package_bool(package, "autotests") {
        add_auto_target_roots(&mut roots, &manifest_dir.join("tests"), true);
    }
    if package_bool(package, "autoexamples") {
        add_auto_target_roots(&mut roots, &manifest_dir.join("examples"), false);
    }
    if package_bool(package, "autobenches") {
        add_auto_target_roots(&mut roots, &manifest_dir.join("benches"), false);
    }

    let mut seen = HashSet::new();
    roots.retain(|root| seen.insert(root.clone()));
    roots
}

fn package_bool(package: Option<&toml::Table>, key: &str) -> bool {
    package
        .and_then(|package| package.get(key))
        .and_then(toml::Value::as_bool)
        .unwrap_or(true)
}

fn add_declared_target_roots(
    roots: &mut Vec<SourceRoot>,
    manifest: Option<&toml::Value>,
    manifest_dir: &Path,
    table: &str,
    default_dir: &str,
    test_only: bool,
) {
    let Some(targets) = manifest
        .and_then(|manifest| manifest.get(table))
        .and_then(toml::Value::as_array)
    else {
        return;
    };
    for target in targets {
        let Some(target) = target.as_table() else {
            continue;
        };
        if let Some(path) = target.get("path").and_then(toml::Value::as_str) {
            add_existing_root(roots, manifest_dir.join(path), test_only);
            continue;
        }
        let Some(name) = target.get("name").and_then(toml::Value::as_str) else {
            continue;
        };
        add_existing_root(
            roots,
            manifest_dir.join(default_dir).join(format!("{name}.rs")),
            test_only,
        );
        add_existing_root(
            roots,
            manifest_dir.join(default_dir).join(name).join("main.rs"),
            test_only,
        );
        if table == "bin" {
            add_existing_root(roots, manifest_dir.join("src/main.rs"), test_only);
        }
    }
}

fn add_source_root(
    roots: &mut Vec<SourceRoot>,
    manifest_dir: &Path,
    configured_path: Option<&str>,
    default_path: &str,
    test_only: bool,
) {
    add_existing_root(
        roots,
        manifest_dir.join(configured_path.unwrap_or(default_path)),
        test_only,
    );
}

fn add_auto_target_roots(roots: &mut Vec<SourceRoot>, directory: &Path, test_only: bool) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|extension| extension == "rs") {
            add_existing_root(roots, path, test_only);
        } else if path.is_dir() {
            add_existing_root(roots, path.join("main.rs"), test_only);
        }
    }
}

fn add_existing_root(roots: &mut Vec<SourceRoot>, path: PathBuf, test_only: bool) {
    if path.is_file() {
        roots.push(SourceRoot {
            path: canonical_path(&path),
            test_only,
        });
    }
}
