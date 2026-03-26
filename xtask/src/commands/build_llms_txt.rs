use std::fs;

use anyhow::Context;

use crate::util::workspace_root;

pub fn run() -> anyhow::Result<()> {
    let workspace_root = workspace_root()?;
    let book_src_dir = workspace_root.join("book").join("src");
    let output_path = workspace_root.join("web").join("public").join("llms.txt");

    println!("Building llms.txt to {}", output_path.display());

    let summary_path = book_src_dir.join("SUMMARY.md");
    let summary_content = fs::read_to_string(&summary_path)
        .with_context(|| format!("Failed to read SUMMARY.md from {}", summary_path.display()))?;

    let mut output = String::new();

    for line in summary_content.lines() {
        if let Some(md_file) = extract_markdown_path(line) {
            let file_path = book_src_dir.join(&md_file);

            if file_path.exists() {
                let content = fs::read_to_string(&file_path)
                    .with_context(|| format!("Failed to read {}", file_path.display()))?;

                output.push_str(&content);
                output.push_str("\n\n---\n\n");
            } else {
                eprintln!("Warning: File not found: {}", file_path.display());
            }
        }
    }

    fs::write(&output_path, output)
        .with_context(|| format!("Failed to write llms.txt to {}", output_path.display()))?;

    println!("llms.txt built successfully");
    Ok(())
}

fn extract_markdown_path(line: &str) -> Option<String> {
    // Parse lines like "- [Title](file.md)" or "  - [Title](file.md)"
    let trimmed = line.trim();
    if !trimmed.starts_with('-') {
        return None;
    }

    let start = trimmed.find('(')?;
    let end = trimmed.find(')')?;

    if start >= end {
        return None;
    }

    Some(trimmed[start + 1..end].to_string())
}
