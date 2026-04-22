use crate::site::i18n::SiteLanguage;
use crate::site::render::{render_page, render_sitemap};
use crate::site::routing::{PageKind, SiteRoute, site_root_prefix};
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

pub fn run() -> Result<()> {
    match std::env::args().nth(1).as_deref() {
        None | Some("build") => build_site(),
        Some(other) => bail!("unsupported command: {other}"),
    }
}

pub fn build_site() -> Result<()> {
    let web_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dist_dir = web_dir.join("dist");
    let bundle_dir = web_dir.join(".dx-bundle");

    if bundle_dir.exists() {
        fs::remove_dir_all(&bundle_dir)
            .with_context(|| format!("failed to remove {}", bundle_dir.display()))?;
    }

    let status = Command::new("dx")
        .current_dir(&web_dir)
        .args(["bundle", "--platform", "web", "--out-dir"])
        .arg(&bundle_dir)
        .status()
        .context("failed to run `dx bundle --web` for the docs site")?;

    if !status.success() {
        bail!("`dx bundle --web` failed with status {status}");
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
    fs::copy(web_dir.join("assets/site.css"), dist_dir.join("site.css")).with_context(|| {
        format!(
            "failed to copy {} to {}",
            web_dir.join("assets/site.css").display(),
            dist_dir.join("site.css").display()
        )
    })?;

    for route in render_routes() {
        let output_dir = route.output_dir();
        let page_dir = dist_dir.join(&output_dir);
        fs::create_dir_all(&page_dir)
            .with_context(|| format!("failed to create {}", page_dir.display()))?;
        let page_html = render_page(route.locale, route.page, &site_root_prefix(&output_dir))?;
        fs::write(page_dir.join("index.html"), page_html).with_context(|| {
            format!("failed to write {}", page_dir.join("index.html").display())
        })?;
    }

    let home_404 = render_page(SiteLanguage::default(), PageKind::Home, "./")?;
    fs::write(dist_dir.join("404.html"), home_404)
        .with_context(|| format!("failed to write {}", dist_dir.join("404.html").display()))?;

    fs::write(dist_dir.join("sitemap.xml"), render_sitemap())
        .with_context(|| format!("failed to write {}", dist_dir.join("sitemap.xml").display()))?;

    fs::remove_dir_all(&bundle_dir)
        .with_context(|| format!("failed to remove {}", bundle_dir.display()))?;

    Ok(())
}

fn render_routes() -> Vec<SiteRoute> {
    let mut routes = Vec::new();
    for locale in SiteLanguage::all() {
        for page in PageKind::all() {
            routes.push(SiteRoute::new(locale, page));
        }
    }
    routes
}

fn copy_directory(source: &Path, destination: &Path) -> Result<()> {
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
