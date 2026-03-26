use std::fs;

use anyhow::bail;

use crate::util::workspace_root;

pub fn run() -> anyhow::Result<()> {
    let workspace_root = workspace_root()?;
    let book_dir = workspace_root.join("book");
    let output_dir = workspace_root.join("web").join("public").join("book");

    println!("Building mdBook to {}", output_dir.display());

    let mut cmd = std::process::Command::new("mdbook");
    cmd.arg("build")
        .current_dir(&book_dir)
        .arg("--dest-dir")
        .arg(&output_dir);

    let status = cmd.status()?;

    if !status.success() {
        bail!("mdbook build failed with status {}", status);
    }

    let gitignore_path = output_dir.join(".gitignore");
    fs::write(&gitignore_path, "*")?;

    println!("mdBook built successfully");
    Ok(())
}
