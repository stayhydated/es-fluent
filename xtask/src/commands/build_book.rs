use std::fs;
use std::path::Path;

use anyhow::bail;

use crate::util::workspace_root;

pub fn run() -> anyhow::Result<()> {
    run_with_paths(
        &workspace_root()?.join("book"),
        &workspace_root()?.join("web").join("public").join("book"),
    )
}

pub fn run_with_paths(book_dir: &Path, output_dir: &Path) -> anyhow::Result<()> {
    println!("Building mdBook to {}", output_dir.display());

    let mut cmd = std::process::Command::new("mdbook");
    cmd.arg("build")
        .current_dir(book_dir)
        .arg("--dest-dir")
        .arg(output_dir);

    let status = cmd.status()?;

    if !status.success() {
        bail!("mdbook build failed with status {}", status);
    }

    let gitignore_path = output_dir.join(".gitignore");
    fs::write(&gitignore_path, "*")?;

    println!("mdBook built successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::run_with_paths;

    #[derive(Debug)]
    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(prefix: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "koruma_xtask_{prefix}_{}_{}",
                std::process::id(),
                nanos
            ));
            fs::create_dir_all(&path).expect("failed to create temp directory");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn write_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("failed to create parent directory");
        }
        fs::write(path, content).expect("failed to write file");
    }

    fn create_minimal_book(book_dir: &Path) {
        let book_toml = r#"[book]
title = "Test Book"
authors = ["Test"]
"#;
        write_file(&book_dir.join("book.toml"), book_toml);

        let summary = "# Summary\n\n- [Test](test.md)\n";
        write_file(&book_dir.join("src").join("SUMMARY.md"), summary);
        write_file(&book_dir.join("src").join("test.md"), "# Test\n\nHello!");
    }

    #[test]
    fn build_book_creates_gitignore_in_output() {
        let tmp = TempDir::new("build_book");
        let book_dir = tmp.path().join("book");
        let output_dir = tmp.path().join("output");

        create_minimal_book(&book_dir);

        // Skip test if mdbook is not installed
        if std::process::Command::new("mdbook")
            .arg("--version")
            .output()
            .is_err()
        {
            eprintln!("Skipping test: mdbook not installed");
            return;
        }

        run_with_paths(&book_dir, &output_dir).expect("build should succeed");

        let gitignore_path = output_dir.join(".gitignore");
        assert!(gitignore_path.exists(), ".gitignore should be created");

        let content = fs::read_to_string(&gitignore_path).expect("failed to read .gitignore");
        assert_eq!(content, "*");
    }

    #[test]
    fn build_book_generates_html_output() {
        let tmp = TempDir::new("build_book_html");
        let book_dir = tmp.path().join("book");
        let output_dir = tmp.path().join("output");

        create_minimal_book(&book_dir);

        if std::process::Command::new("mdbook")
            .arg("--version")
            .output()
            .is_err()
        {
            eprintln!("Skipping test: mdbook not installed");
            return;
        }

        run_with_paths(&book_dir, &output_dir).expect("build should succeed");

        assert!(
            output_dir.join("index.html").exists(),
            "index.html should exist"
        );
    }

    #[test]
    fn build_book_fails_for_invalid_book_dir() {
        let tmp = TempDir::new("build_book_invalid");
        let book_dir = tmp.path().join("nonexistent");
        let output_dir = tmp.path().join("output");

        if std::process::Command::new("mdbook")
            .arg("--version")
            .output()
            .is_err()
        {
            eprintln!("Skipping test: mdbook not installed");
            return;
        }

        let result = run_with_paths(&book_dir, &output_dir);
        assert!(
            result.is_err(),
            "should fail for nonexistent book directory"
        );
    }
}
