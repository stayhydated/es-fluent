//! The current package's i18n inputs for macro expansion.

use es_fluent_build::manifest::{self, BuildManifest, PackageResources};
use es_fluent_shared::LanguageIdentifier;
use es_fluent_shared::resource::{BUILD_MANIFEST_ENV, BUILD_MANIFEST_HASH_ENV};
use es_fluent_toml::{I18nConfig, I18nConfigError};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Where macros read the current package's i18n inputs from.
#[derive(Clone, Debug)]
pub enum PackageInputs {
    /// The manifest written by `es_fluent_build::track_i18n_assets`.
    Manifest(Arc<BuildManifest>),
    /// The package has no build manifest, so its files are read directly.
    Files {
        /// `CARGO_MANIFEST_DIR`, when set.
        manifest_dir: Option<PathBuf>,
    },
}

/// A build manifest that could not be read or parsed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManifestReadError {
    /// Manifest path from the build script.
    pub path: PathBuf,
    /// Failure details.
    pub details: String,
}

impl std::fmt::Display for ManifestReadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "failed to read the es-fluent build manifest at `{}`: {}\nhelp: rerun the package build with matching es-fluent and es-fluent-build versions",
            self.path.display(),
            self.details
        )
    }
}

impl PackageInputs {
    /// Loads the inputs of the package being compiled.
    pub fn current() -> Result<Self, ManifestReadError> {
        match std::env::var_os(BUILD_MANIFEST_ENV) {
            Some(path) => read_manifest(PathBuf::from(path)).map(Self::Manifest),
            None => Ok(Self::Files {
                manifest_dir: std::env::var_os("CARGO_MANIFEST_DIR").map(PathBuf::from),
            }),
        }
    }

    /// Returns the package configuration.
    pub fn config(&self) -> Result<I18nConfig, I18nConfigError> {
        match self {
            Self::Manifest(manifest) => manifest.config(),
            Self::Files {
                manifest_dir: Some(manifest_dir),
            } => I18nConfig::from_manifest_dir(manifest_dir),
            Self::Files { manifest_dir: None } => Err(I18nConfigError::NotFound),
        }
    }

    /// Returns the locales under the configured assets directory.
    pub fn languages(&self, config: &I18nConfig) -> Result<Vec<LanguageIdentifier>, String> {
        match self {
            Self::Manifest(manifest) => manifest.languages(),
            Self::Files { manifest_dir } => {
                manifest::discover_languages(config, &files_manifest_dir(manifest_dir.as_deref())?)
            },
        }
    }

    /// Returns the resources of every package-owned domain.
    pub fn resources(
        &self,
        config: &I18nConfig,
        package: &str,
    ) -> Result<PackageResources, String> {
        match self {
            Self::Manifest(manifest) => manifest.resources(),
            Self::Files { manifest_dir } => manifest::discover_resources(
                config,
                &files_manifest_dir(manifest_dir.as_deref())?,
                package,
            ),
        }
    }
}

/// Returns the current package's configuration, `None` without `i18n.toml`.
pub fn current_package_config() -> Result<Option<I18nConfig>, String> {
    let inputs = PackageInputs::current().map_err(|error| error.to_string())?;
    match inputs.config() {
        Ok(config) => Ok(Some(config)),
        Err(I18nConfigError::NotFound) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

fn files_manifest_dir(manifest_dir: Option<&Path>) -> Result<PathBuf, String> {
    manifest_dir
        .map(Path::to_path_buf)
        .ok_or_else(|| "CARGO_MANIFEST_DIR must be set".to_string())
}

/// Reads a manifest, reusing the parsed value while its content hash is
/// unchanged. A long-lived proc-macro server sees many packages and edits, so
/// the cache keeps one entry per manifest path.
fn read_manifest(path: PathBuf) -> Result<Arc<BuildManifest>, ManifestReadError> {
    use std::{
        collections::HashMap,
        ffi::OsString,
        sync::{LazyLock, Mutex, PoisonError},
    };

    static CACHE: LazyLock<Mutex<HashMap<PathBuf, (OsString, Arc<BuildManifest>)>>> =
        LazyLock::new(Mutex::default);

    let hash = std::env::var_os(BUILD_MANIFEST_HASH_ENV);
    if let Some(hash) = &hash
        && let Some((cached_hash, manifest)) = CACHE
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&path)
        && cached_hash == hash
    {
        return Ok(Arc::clone(manifest));
    }

    let manifest = std::fs::read(&path)
        .map_err(|error| error.to_string())
        .and_then(|bytes| BuildManifest::from_bytes(&bytes))
        .map(Arc::new)
        .map_err(|details| ManifestReadError {
            path: path.clone(),
            details,
        })?;
    if let Some(hash) = hash {
        CACHE
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(path, (hash, Arc::clone(&manifest)));
    }
    Ok(manifest)
}
