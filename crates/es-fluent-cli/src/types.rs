use std::path::PathBuf;
use std::time::Duration;

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
    /// Feature flags that enable es-fluent derives in the crate.
    pub fluent_features: Vec<String>,
}

/// Result of generating FTL for a single crate.
#[derive(Clone, Debug)]
pub struct GenerateResult {
    /// The name of the crate.
    pub name: String,
    /// How long the generation took.
    pub duration: Duration,
    /// Number of FTL resource keys generated.
    pub resource_count: usize,
    /// Error message if generation failed.
    pub error: Option<String>,
}

impl GenerateResult {
    /// Create a new successful result.
    pub fn success(name: String, duration: Duration, resource_count: usize) -> Self {
        Self {
            name,
            duration,
            resource_count,
            error: None,
        }
    }

    /// Create a new error result.
    pub fn failure(name: String, duration: Duration, error: String) -> Self {
        Self {
            name,
            duration,
            resource_count: 0,
            error: Some(error),
        }
    }
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
