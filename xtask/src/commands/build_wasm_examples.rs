use std::{fs, path::Path, process::Command};

use anyhow::{bail, Context};
use walkdir::WalkDir;

use crate::{
    util::workspace_root,
    wasm_examples::{load_manifest, manifest_path, resolve_workspace_path, WasmExample},
};

pub fn run() -> anyhow::Result<()> {
    run_from_workspace_root(&workspace_root()?)
}

fn run_from_workspace_root(workspace_root: &Path) -> anyhow::Result<()> {
    let manifest = load_manifest(workspace_root)?;
    println!(
        "Building {} wasm example(s) from {}",
        manifest.examples.len(),
        manifest_path(workspace_root).display()
    );

    for example in &manifest.examples {
        build_example(workspace_root, example)?;
    }

    Ok(())
}

fn build_example(workspace_root: &Path, example: &WasmExample) -> anyhow::Result<()> {
    let crate_dir = resolve_workspace_path(workspace_root, &example.crate_dir)?;
    let out_dir = resolve_workspace_path(workspace_root, &example.out_dir)?;

    println!(
        "Building wasm example '{}' to {}",
        example.id,
        out_dir.display()
    );

    if out_dir.exists() {
        fs::remove_dir_all(&out_dir)
            .with_context(|| format!("failed to clean {}", out_dir.display()))?;
    }
    fs::create_dir_all(&out_dir)
        .with_context(|| format!("failed to create {}", out_dir.display()))?;
    write_gitignore(&out_dir)?;

    let status = Command::new("wasm-pack")
        .current_dir(&crate_dir)
        .arg("build")
        .args(&example.wasm_pack_args)
        .arg("--out-dir")
        .arg(&out_dir)
        .arg("--out-name")
        .arg(&example.out_name)
        .status()
        .with_context(|| format!("failed to run wasm-pack for '{}'", example.id))?;

    if !status.success() {
        bail!(
            "wasm-pack build failed for '{}' with status {}",
            example.id,
            status
        );
    }

    for copy_path in &example.copy {
        let source = resolve_workspace_path(workspace_root, &copy_path.source)?;
        let destination = resolve_workspace_path(workspace_root, &copy_path.destination)?;
        copy_path_recursive(&source, &destination)?;
    }

    Ok(())
}

fn write_gitignore(out_dir: &Path) -> anyhow::Result<()> {
    fs::write(out_dir.join(".gitignore"), "*\n")
        .with_context(|| format!("failed to write {}", out_dir.join(".gitignore").display()))
}

fn copy_path_recursive(source: &Path, destination: &Path) -> anyhow::Result<()> {
    if source.is_file() {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::copy(source, destination).with_context(|| {
            format!(
                "failed to copy file from {} to {}",
                source.display(),
                destination.display()
            )
        })?;
        return Ok(());
    }

    if !source.is_dir() {
        bail!("copy source does not exist: {}", source.display());
    }

    for entry in WalkDir::new(source) {
        let entry = entry.with_context(|| format!("failed walking {}", source.display()))?;
        let relative_path = entry
            .path()
            .strip_prefix(source)
            .with_context(|| format!("failed to strip prefix {}", source.display()))?;
        let target_path = destination.join(relative_path);

        if entry.file_type().is_dir() {
            fs::create_dir_all(&target_path)
                .with_context(|| format!("failed to create {}", target_path.display()))?;
            continue;
        }

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }

        fs::copy(entry.path(), &target_path).with_context(|| {
            format!(
                "failed to copy file from {} to {}",
                entry.path().display(),
                target_path.display()
            )
        })?;
    }

    Ok(())
}
