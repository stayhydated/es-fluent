use std::fs;
use std::path::Path;

use mdbook::MDBook;

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

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{run_from_workspace_root, run_with_paths};

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

        run_with_paths(&book_dir, &output_dir).expect("build should succeed");

        assert!(
            output_dir.join("index.html").exists(),
            "index.html should exist"
        );
    }

    #[test]
    fn build_book_fails_when_book_is_invalid() {
        let tmp = TempDir::new("build_book_invalid");
        let book_dir = tmp.path().join("book");
        let output_dir = tmp.path().join("output");

        // Missing SUMMARY.md should fail during load/build.
        write_file(
            &book_dir.join("book.toml"),
            r#"[book]
title = "Test Book"
authors = ["Test"]
"#,
        );

        let result = run_with_paths(&book_dir, &output_dir);
        assert!(result.is_err(), "should fail when the book is invalid");
    }

    #[test]
    fn run_from_workspace_root_uses_expected_default_paths() {
        let tmp = TempDir::new("build_book_workspace_root");
        let workspace_root = tmp.path().join("workspace");
        let book_dir = workspace_root.join("book");
        let web_book_output = workspace_root.join("web").join("public").join("book");

        create_minimal_book(&book_dir);
        run_from_workspace_root(&workspace_root).expect("run from workspace root should succeed");

        assert!(
            web_book_output.join(".gitignore").exists(),
            "run() path resolution should write to web/public/book"
        );
    }
}
