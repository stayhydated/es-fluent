use std::fs;
use std::path::Path;

use anyhow::Context;
use mdbook_driver::MDBook;
use mdbook_driver::book::BookItem;
use path_slash::PathExt as _;

const BASE_URL: &str = "https://stayhydated.github.io/es-fluent";
const LLMS_HEADER: &str = include_str!("../../templates/llms-header.md");
const LLMS_MARKDOWN_DIR_NAME: &str = "llms";

pub fn run() -> anyhow::Result<()> {
    run_from_workspace_root(&crate::util::workspace_root()?)
}

fn run_from_workspace_root(workspace_root: &Path) -> anyhow::Result<()> {
    let output_dir = workspace_root.join("web").join("public");
    run_with_paths(
        &workspace_root.join("book"),
        &output_dir.join("llms.txt"),
        &output_dir.join("llms-full.txt"),
        &output_dir.join(LLMS_MARKDOWN_DIR_NAME),
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

fn book_markdown_path(path: &Path) -> anyhow::Result<String> {
    path.to_slash()
        .map(|path| path.into_owned())
        .with_context(|| format!("Book chapter path is not valid UTF-8: {}", path.display()))
}

pub fn run_with_paths(
    book_root: &Path,
    llms_path: &Path,
    llms_full_path: &Path,
    llms_markdown_dir: &Path,
) -> anyhow::Result<()> {
    println!("Building llms.txt to {}", llms_path.display());
    println!("Building llms-full.txt to {}", llms_full_path.display());
    println!(
        "Building llms Markdown files to {}",
        llms_markdown_dir.display()
    );

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
                path: book_markdown_path(path)?,
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
    write_llms_markdown_files(&chapters, llms_markdown_dir)?;

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

fn write_llms_markdown_files(
    chapters: &[ChapterInfo],
    llms_markdown_dir: &Path,
) -> anyhow::Result<()> {
    if llms_markdown_dir.exists() {
        fs::remove_dir_all(llms_markdown_dir).with_context(|| {
            format!(
                "Failed to clear existing llms Markdown directory {}",
                llms_markdown_dir.display()
            )
        })?;
    }

    fs::create_dir_all(llms_markdown_dir).with_context(|| {
        format!(
            "Failed to create llms Markdown directory {}",
            llms_markdown_dir.display()
        )
    })?;

    for chapter in chapters {
        let path = llms_markdown_dir.join(&chapter.path);
        ensure_parent_dir(&path)?;
        fs::write(&path, &chapter.content)
            .with_context(|| format!("Failed to write llms Markdown file {}", path.display()))?;
    }

    Ok(())
}

fn build_llms_txt(chapters: &[ChapterInfo]) -> String {
    let mut output = String::new();
    output.push_str(LLMS_HEADER);
    output.push_str("\n## Docs\n\n");

    for chapter in chapters {
        let url = format!("{}/{}/{}", BASE_URL, LLMS_MARKDOWN_DIR_NAME, chapter.path);
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
