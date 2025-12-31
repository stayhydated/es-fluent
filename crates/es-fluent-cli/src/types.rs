use std::path::PathBuf;

/// Information about a crate that uses es-fluent.
#[derive(Clone, Debug)]
pub struct CrateInfo {
    /// The name of the crate.
    pub name: String,
    /// The path to the crate's manifest directory.
    pub manifest_dir: PathBuf,
    /// The path to the crate's src directory.
    pub src_dir: PathBuf,
    /// The path to the i18n.toml config file.
    pub i18n_config_path: PathBuf,
    /// The path to the FTL output directory (e.g., assets/i18n/en).
    pub ftl_output_dir: PathBuf,
    /// Whether the crate has a lib.rs (required for inventory linking).
    pub has_lib_rs: bool,
    /// Optional feature flag that enables es-fluent derives in the crate.
    pub fluent_feature: Option<String>,
}

/// The state of a crate in the workspace (used by TUI).
#[derive(Clone, Debug)]
pub enum CrateState {
    /// The crate is missing lib.rs, so generation cannot work.
    MissingLibRs,
    /// FTL files are currently being generated.
    Generating,
    /// Watching for changes. Contains the count of FTL resources.
    Watching {
        /// Number of FTL resource keys in this crate.
        resource_count: usize,
    },
    /// Generation failed with an error.
    Error {
        /// The error message.
        message: String,
    },
}
