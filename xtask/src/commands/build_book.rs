use std::fs;
use std::path::Path;

use mdbook_driver::MDBook;

use crate::util::workspace_root;

pub fn run() -> anyhow::Result<()> {
    run_from_workspace_root(&workspace_root()?)
}

fn run_from_workspace_root(workspace_root: &Path) -> anyhow::Result<()> {
    run_with_paths(
        &workspace_root.join("book"),
        &workspace_root.join("web").join("public").join("book"),
    )
}

pub fn run_with_paths(book_dir: &Path, output_dir: &Path) -> anyhow::Result<()> {
    println!("Building mdBook to {}", output_dir.display());

    let mut book = MDBook::load(book_dir)?;
    book.config.build.build_dir = output_dir.to_path_buf();
    book.build()?;

    let gitignore_path = output_dir.join(".gitignore");
    fs::write(&gitignore_path, "*")?;

    println!("mdBook built successfully");
    Ok(())
}
