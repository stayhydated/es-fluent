use std::{fs, path::Path};

use anyhow::{bail, Context};

use crate::util::workspace_root;

struct WasmMarkerCheck {
    label: &'static str,
    wasm_path: &'static str,
    markers: &'static [&'static str],
}

const WASM_MARKER_CHECKS: &[WasmMarkerCheck] = &[WasmMarkerCheck {
    label: "bevy-example localized language inventory",
    wasm_path: "web/public/bevy-example/bevy-example_bg.wasm",
    markers: &["es-fluent-lang-en"],
}];

pub fn run() -> anyhow::Result<()> {
    run_from_workspace_root(&workspace_root()?)
}

fn run_from_workspace_root(workspace_root: &Path) -> anyhow::Result<()> {
    println!(
        "Verifying {} repo-specific wasm marker check(s)",
        WASM_MARKER_CHECKS.len()
    );

    let mut failures = Vec::new();

    for check in WASM_MARKER_CHECKS {
        verify_marker_check(workspace_root, check, &mut failures)?;
    }

    if !failures.is_empty() {
        bail!(
            "Wasm marker verification failed:\n{}",
            failures
                .iter()
                .map(|message| format!("- {message}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    println!(
        "Verified repo-specific wasm markers for {} check(s)",
        WASM_MARKER_CHECKS.len()
    );
    Ok(())
}

fn verify_marker_check(
    workspace_root: &Path,
    check: &WasmMarkerCheck,
    failures: &mut Vec<String>,
) -> anyhow::Result<()> {
    let wasm_path = workspace_root.join(check.wasm_path);

    if !wasm_path.is_file() {
        failures.push(format!(
            "{} missing wasm output {}",
            check.label,
            wasm_path.display()
        ));
        return Ok(());
    }

    let wasm_bytes = fs::read(&wasm_path)
        .with_context(|| format!("failed to read wasm output at {}", wasm_path.display()))?;

    for marker in check.markers {
        if !contains_bytes(&wasm_bytes, marker.as_bytes()) {
            failures.push(format!(
                "{} missing '{}' in {}",
                check.label,
                marker,
                wasm_path.display()
            ));
        }
    }

    Ok(())
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}
