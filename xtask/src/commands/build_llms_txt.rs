use std::fs;
use std::path::Path;

use anyhow::Context;
use mdbook_driver::book::BookItem;
use mdbook_driver::MDBook;
use path_slash::PathExt as _;

use crate::util::workspace_root;

const BASE_URL: &str = "https://stayhydated.github.io/es-fluent";
const LLMS_HEADER: &str = include_str!("../../templates/llms-header.md");

pub fn run() -> anyhow::Result<()> {
    run_from_workspace_root(&workspace_root()?)
}

fn run_from_workspace_root(workspace_root: &Path) -> anyhow::Result<()> {
    let output_dir = workspace_root.join("web").join("public");
    run_with_paths(
        &workspace_root.join("book"),
        &output_dir.join("llms.txt"),
        &output_dir.join("llms-full.txt"),
    )
}

/// Chapter metadata extracted from the mdBook.
struct ChapterInfo {
    name: String,
    path: String,
    content: String,
}

fn ensure_parent_dir(path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output directory {}", parent.display()))?;
    }
    Ok(())
}

fn book_html_path(path: &Path) -> anyhow::Result<String> {
    path.with_extension("html")
        .to_slash()
        .map(|path| path.into_owned())
        .with_context(|| format!("Book chapter path is not valid UTF-8: {}", path.display()))
}

pub fn run_with_paths(
    book_root: &Path,
    llms_path: &Path,
    llms_full_path: &Path,
) -> anyhow::Result<()> {
    println!("Building llms.txt to {}", llms_path.display());
    println!("Building llms-full.txt to {}", llms_full_path.display());

    let mdbook = MDBook::load(book_root)
        .with_context(|| format!("Failed to load book from {}", book_root.display()))?;

    let chapters: Vec<ChapterInfo> = mdbook
        .iter()
        .filter_map(|item| match item {
            BookItem::Chapter(chapter) if !chapter.is_draft_chapter() => Some(chapter),
            _ => None,
        })
        .map(|chapter| {
            let path = chapter
                .path
                .as_ref()
                .with_context(|| format!("Missing path for book chapter '{}'", chapter.name))?;

            Ok(ChapterInfo {
                name: chapter.name.clone(),
                path: book_html_path(path)?,
                content: chapter.content.clone(),
            })
        })
        .collect::<anyhow::Result<_>>()?;

    // Build llms.txt (structured index with links)
    let llms_txt = build_llms_txt(&chapters);

    // Build llms-full.txt (expanded content)
    let llms_full_txt = build_llms_full_txt(&chapters);

    ensure_parent_dir(llms_path)?;
    ensure_parent_dir(llms_full_path)?;

    fs::write(llms_path, llms_txt)
        .with_context(|| format!("Failed to write llms.txt to {}", llms_path.display()))?;

    fs::write(llms_full_path, llms_full_txt).with_context(|| {
        format!(
            "Failed to write llms-full.txt to {}",
            llms_full_path.display()
        )
    })?;

    println!("llms.txt and llms-full.txt built successfully");
    Ok(())
}

fn build_llms_txt(chapters: &[ChapterInfo]) -> String {
    let mut output = String::new();
    output.push_str(LLMS_HEADER);
    output.push_str("\n## Docs\n\n");

    for chapter in chapters {
        let url = format!("{}/book/{}", BASE_URL, chapter.path);
        output.push_str(&format!("- [{}]({})\n", chapter.name, url));
    }

    output
}

fn build_llms_full_txt(chapters: &[ChapterInfo]) -> String {
    let mut output = String::new();
    output.push_str(LLMS_HEADER);
    output.push_str("\n## Docs\n\n");

    for chapter in chapters {
        output.push_str(&chapter.content);
        output.push_str("\n\n---\n\n");
    }

    output
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::{book_html_path, run_from_workspace_root, run_with_paths};

    fn write_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("failed to create parent directory");
        }
        fs::write(path, content).expect("failed to write file");
    }

    fn create_book_toml(book_root: &Path) {
        let toml = r#"[book]
title = "Test Book"
"#;
        write_file(&book_root.join("book.toml"), toml);
    }

    #[test]
    fn llms_txt_contains_structured_index() {
        let tmp = tempfile::tempdir().expect("failed to create temp directory");
        let book_root = tmp.path().join("book");
        let book_src = book_root.join("src");
        let llms_path = tmp.path().join("web").join("public").join("llms.txt");
        let llms_full_path = tmp.path().join("web").join("public").join("llms-full.txt");

        create_book_toml(&book_root);

        let summary = r#"# Summary

- [Intro](intro.md)
- [Guide](guide.md)
"#;

        write_file(&book_src.join("SUMMARY.md"), summary);
        write_file(&book_src.join("intro.md"), "# Introduction\n\nWelcome!");
        write_file(&book_src.join("guide.md"), "# Guide\n\nStep by step.");

        run_with_paths(&book_root, &llms_path, &llms_full_path)
            .expect("run_with_paths should succeed");

        let result = fs::read_to_string(&llms_path).expect("failed to read llms.txt");

        // Should have header content from LLMS_HEADER template
        assert!(!result.is_empty(), "llms.txt should not be empty");
        assert!(
            result.starts_with('#'),
            "should start with markdown heading"
        );

        // Should have Docs section with links
        assert!(result.contains("## Docs"));
        assert!(result.contains("[Intro]("));
        assert!(result.contains("intro.html)"));
        assert!(result.contains("[Guide]("));
        assert!(result.contains("guide.html)"));

        // Should NOT contain full content inline
        assert!(
            !result.contains("Welcome!"),
            "llms.txt should not contain chapter content"
        );
    }

    #[test]
    fn llms_full_txt_contains_expanded_content() {
        let tmp = tempfile::tempdir().expect("failed to create temp directory");
        let book_root = tmp.path().join("book");
        let book_src = book_root.join("src");
        let llms_path = tmp.path().join("web").join("public").join("llms.txt");
        let llms_full_path = tmp.path().join("web").join("public").join("llms-full.txt");

        create_book_toml(&book_root);

        let summary = r#"# Summary

- [Intro](intro.md)
- [Guide](guide.md)
"#;

        write_file(&book_src.join("SUMMARY.md"), summary);
        write_file(&book_src.join("intro.md"), "# Introduction\n\nWelcome!");
        write_file(&book_src.join("guide.md"), "# Guide\n\nStep by step.");

        run_with_paths(&book_root, &llms_path, &llms_full_path)
            .expect("run_with_paths should succeed");

        let result = fs::read_to_string(&llms_full_path).expect("failed to read llms-full.txt");

        // Should have header content from LLMS_HEADER template
        assert!(!result.is_empty(), "llms-full.txt should not be empty");
        assert!(
            result.starts_with('#'),
            "should start with markdown heading"
        );

        // Should contain full chapter content
        assert!(result.contains("# Introduction"));
        assert!(result.contains("Welcome!"));
        assert!(result.contains("# Guide"));
        assert!(result.contains("Step by step."));
        assert!(result.contains("\n\n---\n\n"));

        let separator_count = result.matches("---").count();
        assert_eq!(separator_count, 2, "expected 2 separators for 2 chapters");
    }

    #[test]
    fn run_with_paths_skips_draft_chapters() {
        let tmp = tempfile::tempdir().expect("failed to create temp directory");
        let book_root = tmp.path().join("book");
        let book_src = book_root.join("src");
        let llms_path = tmp.path().join("output").join("llms.txt");
        let llms_full_path = tmp.path().join("output").join("llms-full.txt");

        create_book_toml(&book_root);

        let summary = r#"# Summary

- [Exists](exists.md)
- [Draft]()
"#;

        write_file(&book_src.join("SUMMARY.md"), summary);
        write_file(&book_src.join("exists.md"), "# Exists\n\nContent here.");

        run_with_paths(&book_root, &llms_path, &llms_full_path)
            .expect("run_with_paths should succeed");

        // Check llms.txt
        let llms_result = fs::read_to_string(&llms_path).expect("failed to read llms.txt");
        assert!(llms_result.contains("[Exists]"));
        assert!(!llms_result.contains("[Draft]"));

        // Check llms-full.txt
        let llms_full_result =
            fs::read_to_string(&llms_full_path).expect("failed to read llms-full.txt");
        assert!(llms_full_result.contains("# Exists"));
        assert!(!llms_full_result.contains("Draft"));

        let separator_count = llms_full_result.matches("---").count();
        assert_eq!(
            separator_count, 1,
            "expected 1 separator for 1 existing file"
        );
    }

    #[test]
    fn run_with_paths_fails_for_missing_book() {
        let tmp = tempfile::tempdir().expect("failed to create temp directory");
        let book_root = tmp.path().join("book");
        let llms_path = tmp.path().join("output").join("llms.txt");
        let llms_full_path = tmp.path().join("output").join("llms-full.txt");

        fs::create_dir_all(&book_root).expect("failed to create book directory");

        let result = run_with_paths(&book_root, &llms_path, &llms_full_path);
        assert!(result.is_err(), "should fail when book.toml is missing");
    }

    #[test]
    fn run_with_paths_creates_output_directory() {
        let tmp = tempfile::tempdir().expect("failed to create temp directory");
        let book_root = tmp.path().join("book");
        let book_src = book_root.join("src");
        let llms_path = tmp.path().join("nested").join("deep").join("llms.txt");
        let llms_full_path = tmp.path().join("nested").join("deep").join("llms-full.txt");

        create_book_toml(&book_root);

        let summary = "# Summary\n\n- [Test](test.md)\n";
        write_file(&book_src.join("SUMMARY.md"), summary);
        write_file(&book_src.join("test.md"), "# Test\n\nContent.");

        run_with_paths(&book_root, &llms_path, &llms_full_path)
            .expect("run_with_paths should succeed");

        assert!(llms_path.exists(), "llms.txt should be created");
        assert!(llms_full_path.exists(), "llms-full.txt should be created");
    }

    #[test]
    fn run_with_paths_creates_both_output_directories_when_parents_differ() {
        let tmp = tempfile::tempdir().expect("failed to create temp directory");
        let book_root = tmp.path().join("book");
        let book_src = book_root.join("src");
        let llms_path = tmp.path().join("output").join("links").join("llms.txt");
        let llms_full_path = tmp
            .path()
            .join("expanded")
            .join("docs")
            .join("llms-full.txt");

        create_book_toml(&book_root);

        let summary = "# Summary\n\n- [Test](test.md)\n";
        write_file(&book_src.join("SUMMARY.md"), summary);
        write_file(&book_src.join("test.md"), "# Test\n\nContent.");

        run_with_paths(&book_root, &llms_path, &llms_full_path)
            .expect("run_with_paths should succeed");

        assert!(llms_path.exists(), "llms.txt should be created");
        assert!(llms_full_path.exists(), "llms-full.txt should be created");
    }

    #[test]
    fn book_html_path_normalizes_windows_separators() {
        use std::path::PathBuf;

        use path_slash::PathBufExt as _;

        let path = PathBuf::from_backslash(r"guide\intro.md");
        let normalized = book_html_path(&path).expect("path should be valid UTF-8");
        assert_eq!(normalized, "guide/intro.html");
    }

    #[test]
    fn run_from_workspace_root_uses_default_workspace_paths() {
        let tmp = tempfile::tempdir().expect("failed to create temp directory");
        let workspace_root = tmp.path().join("workspace");
        let book_root = workspace_root.join("book");
        let book_src = book_root.join("src");
        let llms_path = workspace_root.join("web").join("public").join("llms.txt");
        let llms_full_path = workspace_root
            .join("web")
            .join("public")
            .join("llms-full.txt");

        create_book_toml(&book_root);

        let summary = "# Summary\n\n- [Test](test.md)\n";
        write_file(&book_src.join("SUMMARY.md"), summary);
        write_file(&book_src.join("test.md"), "# Test\n\nWorkspace root mode.");

        run_from_workspace_root(&workspace_root).expect("run should succeed");

        // Check llms.txt has link
        let llms_result = fs::read_to_string(&llms_path).expect("failed to read llms.txt");
        assert!(
            llms_result.contains("[Test]"),
            "llms.txt should contain link to chapter"
        );

        // Check llms-full.txt has content
        let llms_full_result =
            fs::read_to_string(&llms_full_path).expect("failed to read llms-full.txt");
        assert!(
            llms_full_result.contains("Workspace root mode."),
            "llms-full.txt should contain chapter content"
        );
    }
}
