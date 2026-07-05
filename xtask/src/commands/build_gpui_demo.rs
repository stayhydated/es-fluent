use std::{fs, path::Path, process::Command};

use anyhow::{Context as _, bail};

const EXAMPLE_DIR: &str = "examples/gpui-example";
const OUTPUT_ROOT: &str = "web/public/gpui-demo";
const OUTPUT_DIR: &str = "web/public/gpui-demo";
const REQUIRED_MARKER: &str = "GpuiScreenMessages";
const NIGHTLY_TOOLCHAIN: &str = "nightly";

pub fn run() -> anyhow::Result<()> {
    run_from_workspace_root(&stayhydated_xtask::workspace_root_from_xtask_manifest()?)
}

fn run_from_workspace_root(workspace_root: &Path) -> anyhow::Result<()> {
    let example_dir = workspace_root.join(EXAMPLE_DIR);
    let output_root = workspace_root.join(OUTPUT_ROOT);
    let output_dir = workspace_root.join(OUTPUT_DIR);

    if !example_dir.join("index.html").is_file() {
        bail!(
            "gpui demo build requires {}",
            example_dir.join("index.html").display()
        );
    }

    println!("Building GPUI demo from {}", example_dir.display());

    if output_root.exists() {
        fs::remove_dir_all(&output_root)
            .with_context(|| format!("failed to clean {}", output_root.display()))?;
    }
    fs::create_dir_all(&output_root)
        .with_context(|| format!("failed to create {}", output_root.display()))?;

    let status = Command::new("trunk")
        .current_dir(&example_dir)
        .env("RUSTUP_TOOLCHAIN", NIGHTLY_TOOLCHAIN)
        .env_remove("NO_COLOR")
        .arg("build")
        .arg("index.html")
        .arg("--example")
        .arg("gpui-example")
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
        .context("failed to run trunk for GPUI demo")?;

    if !status.success() {
        bail!(
            "trunk build failed for GPUI demo with status {status}. Ensure nightly is installed (`rustup toolchain install nightly`)."
        );
    }
    verify_output(&output_dir)?;
    write_gitignore(&output_root)?;

    Ok(())
}

fn verify_output(output_dir: &Path) -> anyhow::Result<()> {
    let mut has_wasm = false;
    let mut has_js = false;

    for entry in fs::read_dir(output_dir)
        .with_context(|| format!("failed to read {}", output_dir.display()))?
    {
        let path = entry?.path();
        if !path.is_file() {
            continue;
        }

        match path.extension().and_then(|extension| extension.to_str()) {
            Some("wasm") => {
                has_wasm = has_wasm || verify_wasm_has_marker(&path)?;
            },
            Some("js") => {
                has_js = true;
            },
            _ => {},
        }
    }

    if !has_wasm {
        bail!("missing GPUI demo wasm output in {}", output_dir.display());
    }

    if !has_js {
        bail!(
            "missing GPUI demo JavaScript output in {}",
            output_dir.display()
        );
    }

    Ok(())
}

fn verify_wasm_has_marker(wasm_path: &Path) -> anyhow::Result<bool> {
    let marker_found = fs::read(wasm_path)
        .map(|bytes| {
            bytes
                .windows(REQUIRED_MARKER.len())
                .any(|w| w == REQUIRED_MARKER.as_bytes())
        })
        .unwrap_or(false);

    Ok(marker_found)
}

fn write_gitignore(output_root: &Path) -> anyhow::Result<()> {
    fs::write(output_root.join(".gitignore"), "*\n").with_context(|| {
        format!(
            "failed to write {}",
            output_root.join(".gitignore").display()
        )
    })
}
