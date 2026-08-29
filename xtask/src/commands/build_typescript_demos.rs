use std::process::Command;

use anyhow::{Context as _, bail};

const DEMO_OUTPUTS: [&str; 4] = ["typescript-demo", "solid-demo", "react-demo", "expo-demo"];

pub fn run() -> anyhow::Result<()> {
    let workspace_root = stayhydated_xtask::workspace_root_from_xtask_manifest()?;
    let npm = if cfg!(windows) { "npm.cmd" } else { "npm" };
    let status = Command::new(npm)
        .args(["run", "build:demos"])
        .current_dir(&workspace_root)
        .status()
        .context("failed to start the TypeScript demo build")?;
    if !status.success() {
        bail!("TypeScript demo build failed with {status}");
    }

    for demo in DEMO_OUTPUTS {
        let index = workspace_root
            .join("web/public")
            .join(demo)
            .join("index.html");
        if !index.is_file() {
            bail!("TypeScript demo build did not produce {}", index.display());
        }
    }
    Ok(())
}
