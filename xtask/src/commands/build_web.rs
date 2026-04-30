use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, bail};
use walkdir::WalkDir;

const BUNDLE_DIR: &str = "web/.dx-bundle";
const DIST_DIR: &str = "web/dist";
const DX_RELEASE_PUBLIC_DIR: &str = "target/dx/web/release/web/public";
const BEVY_DEMO_DIR: &str = "web/public/bevy-demo";
const ASSETS_DIR: &str = "web/assets";
const SITE_CSS: &str = "web/public/assets/site.css";
const BOOK_DIR: &str = "web/public/book";
const LLMS_FULL_TXT: &str = "web/public/llms-full.txt";
const LLMS_TXT: &str = "web/public/llms.txt";
const ROOT_NOJEKYLL: &str = "web/public/.nojekyll";

pub fn run() -> anyhow::Result<()> {
    run_from_workspace_root(&crate::util::workspace_root()?)
}

fn run_from_workspace_root(workspace_root: &Path) -> anyhow::Result<()> {
    let web_dir = workspace_root.join("web");
    let bundle_dir = workspace_root.join(BUNDLE_DIR);
    let dist_dir = workspace_root.join(DIST_DIR);
    let dx_release_public_dir = workspace_root.join(DX_RELEASE_PUBLIC_DIR);

    if bundle_dir.exists() {
        fs::remove_dir_all(&bundle_dir)
            .with_context(|| format!("failed to remove {}", bundle_dir.display()))?;
    }

    web::cleanup_generated_route_cache(&dx_release_public_dir).with_context(|| {
        format!(
            "failed to clear generated route cache at {}",
            dx_release_public_dir.display()
        )
    })?;

    let status = Command::new("dx")
        .current_dir(&web_dir)
        .args(["bundle", "--platform", "web", "--ssg", "--out-dir"])
        .arg(&bundle_dir)
        .status()
        .context("failed to run `dx bundle --platform web --ssg` for the docs site")?;

    if !status.success() {
        bail!("`dx bundle --platform web --ssg` failed with status {status}");
    }

    let bundled_public = bundle_dir.join("public");
    if !bundled_public.is_dir() {
        bail!(
            "expected bundled static output at {}",
            bundled_public.display()
        );
    }

    if dist_dir.exists() {
        fs::remove_dir_all(&dist_dir)
            .with_context(|| format!("failed to remove {}", dist_dir.display()))?;
    }
    fs::create_dir_all(&dist_dir)
        .with_context(|| format!("failed to create {}", dist_dir.display()))?;

    copy_directory(&bundled_public, &dist_dir)?;
    copy_directory(&workspace_root.join(ASSETS_DIR), &dist_dir.join("assets"))?;
    copy_file_if_present(
        &workspace_root.join(SITE_CSS),
        &dist_dir.join("assets").join("site.css"),
    )?;
    copy_directory(&workspace_root.join(BOOK_DIR), &dist_dir.join("book"))?;
    copy_directory(
        &workspace_root.join(BEVY_DEMO_DIR),
        &dist_dir.join("bevy-demo"),
    )?;
    copy_file_if_present(
        &workspace_root.join(ROOT_NOJEKYLL),
        &dist_dir.join(".nojekyll"),
    )?;
    copy_file_if_present(&workspace_root.join(LLMS_TXT), &dist_dir.join("llms.txt"))?;
    copy_file_if_present(
        &workspace_root.join(LLMS_FULL_TXT),
        &dist_dir.join("llms-full.txt"),
    )?;
    fs::copy(dist_dir.join("index.html"), dist_dir.join("404.html"))
        .with_context(|| format!("failed to write {}", dist_dir.join("404.html").display()))?;
    fs::write(dist_dir.join("sitemap.xml"), web::sitemap_xml())
        .with_context(|| format!("failed to write {}", dist_dir.join("sitemap.xml").display()))?;

    fs::remove_dir_all(&bundle_dir)
        .with_context(|| format!("failed to remove {}", bundle_dir.display()))?;

    Ok(())
}

fn copy_file_if_present(source: &Path, destination: &Path) -> anyhow::Result<()> {
    if !source.is_file() {
        return Ok(());
    }

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    fs::copy(source, destination).with_context(|| {
        format!(
            "failed to copy {} to {}",
            source.display(),
            destination.display()
        )
    })?;

    Ok(())
}

fn copy_directory(source: &Path, destination: &Path) -> anyhow::Result<()> {
    if !source.exists() {
        return Ok(());
    }

    for entry in WalkDir::new(source) {
        let entry = entry.with_context(|| format!("failed to walk {}", source.display()))?;
        let relative = entry
            .path()
            .strip_prefix(source)
            .with_context(|| format!("failed to strip prefix {}", source.display()))?;

        if relative.as_os_str().is_empty() {
            continue;
        }

        let target = destination.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)
                .with_context(|| format!("failed to create {}", target.display()))?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create {}", parent.display()))?;
            }
            fs::copy(entry.path(), &target).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    entry.path().display(),
                    target.display()
                )
            })?;
        }
    }

    Ok(())
}
