//! Inventory collection functionality for CLI commands.

use serde::Serialize;
use std::collections::{HashMap, HashSet};

/// Expected key information from inventory.
#[derive(Serialize)]
pub struct ExpectedKey {
    pub key: String,
    pub variables: Vec<String>,
}

/// The inventory data output.
#[derive(Serialize)]
pub struct InventoryData {
    pub expected_keys: Vec<ExpectedKey>,
}

/// Collects inventory data for a crate and writes it to `inventory.json`.
///
/// This function is used by the es-fluent CLI to collect expected FTL keys
/// and their variables from inventory-registered types.
///
/// # Arguments
///
/// * `crate_name` - The name of the crate to collect inventory for (e.g., "my-crate")
///
/// # Panics
///
/// Panics if serialization or file writing fails.
pub fn write_inventory_for_crate(crate_name: &str) {
    let crate_ident = crate_name.replace('-', "_");

    // Collect all registered type infos for this crate
    let type_infos: Vec<_> = es_fluent_core::registry::get_all_ftl_type_infos()
        .into_iter()
        .filter(|info| {
            info.module_path == crate_ident
                || info.module_path.starts_with(&format!("{}::", crate_ident))
        })
        .collect();

    // Build a map of expected keys and their required variables (deduplicated)
    let mut keys_map: HashMap<String, HashSet<String>> = HashMap::new();
    for info in &type_infos {
        for variant in &info.variants {
            let key = variant.ftl_key.0.clone();
            let vars: HashSet<String> = variant.args.iter().cloned().collect();
            keys_map.entry(key).or_default().extend(vars);
        }
    }

    // Convert to output format
    let expected_keys: Vec<ExpectedKey> = keys_map
        .into_iter()
        .map(|(key, vars)| ExpectedKey {
            key,
            variables: vars.into_iter().collect(),
        })
        .collect();

    let data = InventoryData { expected_keys };

    // Write inventory data to file
    let json = serde_json::to_string(&data).expect("Failed to serialize inventory data");
    std::fs::write("inventory.json", json).expect("Failed to write inventory.json");
}
