use std::{fs, path::Path, process::Command};

use anyhow::{bail, Context};

use crate::util::workspace_root;

const EXAMPLE_DIR: &str = "examples/bevy-example";
const OUTPUT_ROOT: &str = "web/public/bevy-demo";
const OUTPUT_DIR: &str = "web/public/bevy-demo";
const REQUIRED_MARKER: &str = "es-fluent-lang-en";

pub fn run() -> anyhow::Result<()> {
    run_from_workspace_root(&workspace_root()?)
}

fn run_from_workspace_root(workspace_root: &Path) -> anyhow::Result<()> {
    let example_dir = workspace_root.join(EXAMPLE_DIR);
    let output_root = workspace_root.join(OUTPUT_ROOT);
    let output_dir = workspace_root.join(OUTPUT_DIR);
    let html_file = example_dir.join("index.html");

    if !html_file.is_file() {
        bail!("bevy demo build requires {}", html_file.display());
    }

    println!("Building Bevy demo from {}", example_dir.display());

    if output_root.exists() {
        fs::remove_dir_all(&output_root)
            .with_context(|| format!("failed to clean {}", output_root.display()))?;
    }
    fs::create_dir_all(&output_root)
        .with_context(|| format!("failed to create {}", output_root.display()))?;

    let status = Command::new("trunk")
        .current_dir(&example_dir)
        .env_remove("NO_COLOR")
        .arg("build")
        .arg("index.html")
        .args([
            "--release",
            "--no-default-features",
            "--no-sri",
            "--public-url",
            "./",
        ])
        .arg("--dist")
        .arg(&output_dir)
        .status()
        .context("failed to run trunk for the Bevy demo")?;

    if !status.success() {
        bail!(
            "trunk build failed for the Bevy demo with status {}",
            status
        );
    }

    verify_output(&output_dir)?;
    write_gitignore(&output_root)?;

    Ok(())
}

fn verify_output(output_dir: &Path) -> anyhow::Result<()> {
    let index_html = output_dir.join("index.html");
    if !index_html.is_file() {
        bail!("missing Bevy demo HTML output at {}", index_html.display());
    }

    let wasm_paths = fs::read_dir(output_dir)
        .with_context(|| format!("failed to read {}", output_dir.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension == "wasm")
        })
        .collect::<Vec<_>>();

    if wasm_paths.is_empty() {
        bail!("missing Bevy demo wasm output in {}", output_dir.display());
    }

    let marker_found = wasm_paths.iter().any(|path| {
        fs::read(path)
            .map(|bytes| {
                bytes
                    .windows(REQUIRED_MARKER.len())
                    .any(|w| w == REQUIRED_MARKER.as_bytes())
            })
            .unwrap_or(false)
    });

    if !marker_found {
        bail!(
            "Bevy demo wasm output in {} is missing '{}'",
            output_dir.display(),
            REQUIRED_MARKER
        );
    }

    Ok(())
}

fn write_gitignore(output_root: &Path) -> anyhow::Result<()> {
    fs::write(output_root.join(".gitignore"), "*\n").with_context(|| {
        format!(
            "failed to write {}",
            output_root.join(".gitignore").display()
        )
    })
}
