use anyhow::Context;
use std::path::{Path, PathBuf};

pub fn workspace_root() -> anyhow::Result<PathBuf> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .map(Path::to_path_buf)
        .context("failed to resolve workspace root from xtask manifest directory")
}
