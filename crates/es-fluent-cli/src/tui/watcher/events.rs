use crate::core::CrateInfo;
use notify_debouncer_full::DebouncedEvent;
use std::collections::HashMap;
use std::path::PathBuf;

pub(super) type PathToCrateMap = HashMap<PathBuf, String>;

pub(super) fn build_path_to_crate(valid_crates: &[&CrateInfo]) -> PathToCrateMap {
    valid_crates
        .iter()
        .map(|krate| (krate.src_dir.clone(), krate.name.clone()))
        .collect()
}

/// Process file events and return the set of affected crate names.
pub(super) fn process_file_events(
    events: &[DebouncedEvent],
    path_to_crate: &PathToCrateMap,
) -> Vec<String> {
    let mut affected: HashMap<String, ()> = HashMap::new();

    for event in events {
        for path in &event.paths {
            if path.components().any(|c| c.as_os_str() == ".es-fluent") {
                continue;
            }

            if path.extension().is_some_and(|ext| ext == "ftl") {
                continue;
            }

            for (src_dir, crate_name) in path_to_crate {
                if path.starts_with(src_dir) || path.ends_with("i18n.toml") {
                    let is_rs = path.extension().is_some_and(|ext| ext == "rs");
                    let is_i18n = path.file_name().is_some_and(|n| n == "i18n.toml");

                    if is_rs || is_i18n {
                        affected.insert(crate_name.clone(), ());
                    }
                    break;
                }
            }
        }
    }

    affected.into_keys().collect()
}
