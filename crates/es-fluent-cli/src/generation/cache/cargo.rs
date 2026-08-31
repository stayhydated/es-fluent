use fs_err as fs;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

#[derive(Debug)]
pub(crate) struct CargoInputs {
    pub(crate) config_paths: BTreeSet<PathBuf>,
    pub(crate) lockfile_paths: BTreeSet<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
struct CargoConfigFile {
    #[serde(default)]
    include: Vec<CargoConfigInclude>,
    #[serde(default)]
    resolver: CargoResolverConfig,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum CargoConfigInclude {
    Path(PathBuf),
    Detailed { path: PathBuf },
}

impl CargoConfigInclude {
    fn into_path(self) -> PathBuf {
        match self {
            Self::Path(path) | Self::Detailed { path } => path,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct CargoResolverConfig {
    #[serde(rename = "lockfile-path")]
    lockfile_path: Option<PathBuf>,
}

pub(crate) fn configured_cargo_home() -> Option<PathBuf> {
    std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo")))
        .or_else(|| std::env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join(".cargo")))
}

fn configured_lockfile_path() -> Option<PathBuf> {
    std::env::var_os("CARGO_RESOLVER_LOCKFILE_PATH")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
}

pub(crate) fn cargo_inputs(workspace_root: &Path, cargo_home: Option<PathBuf>) -> CargoInputs {
    let mut config_dirs = workspace_root
        .ancestors()
        .map(|ancestor| ancestor.join(".cargo"))
        .collect::<BTreeSet<_>>();
    if let Some(cargo_home) = cargo_home {
        config_dirs.insert(if cargo_home.is_absolute() {
            cargo_home
        } else {
            workspace_root.join(cargo_home)
        });
    }

    let mut pending_config_paths = config_dirs
        .iter()
        .flat_map(|directory| [directory.join("config.toml"), directory.join("config")])
        .collect::<Vec<_>>();
    let mut config_paths = BTreeSet::new();
    let mut lockfile_paths = BTreeSet::from([workspace_root.join("Cargo.lock")]);

    if let Some(lockfile_path) = configured_lockfile_path() {
        if lockfile_path.is_absolute() {
            lockfile_paths.insert(normalize_lexical_path(&lockfile_path));
        } else {
            lockfile_paths.insert(resolve_cargo_path(workspace_root, &lockfile_path));
            lockfile_paths.insert(resolve_cargo_path(
                &workspace_root.join(".es-fluent"),
                &lockfile_path,
            ));
            if let Ok(current_dir) = std::env::current_dir() {
                lockfile_paths.insert(resolve_cargo_path(&current_dir, &lockfile_path));
            }
        }
    }

    while let Some(config_path) = pending_config_paths.pop() {
        let config_path = normalize_lexical_path(&config_path);
        if !config_paths.insert(config_path.clone()) {
            continue;
        }

        let Ok(source) = fs::read_to_string(&config_path) else {
            continue;
        };
        let Ok(config) = toml::from_str::<CargoConfigFile>(&source) else {
            continue;
        };
        let config_dir = config_path.parent().unwrap_or(workspace_root);

        pending_config_paths.extend(
            config
                .include
                .into_iter()
                .map(CargoConfigInclude::into_path)
                .map(|path| resolve_cargo_path(config_dir, &path)),
        );
        if let Some(lockfile_path) = config.resolver.lockfile_path {
            let config_value_base = config_dir.parent().unwrap_or(config_dir);
            lockfile_paths.insert(resolve_cargo_path(config_value_base, &lockfile_path));
        }
    }

    CargoInputs {
        config_paths,
        lockfile_paths,
    }
}

fn resolve_cargo_path(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        normalize_lexical_path(path)
    } else {
        normalize_lexical_path(&base.join(path))
    }
}

fn normalize_lexical_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {},
            Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push(component.as_os_str());
                }
            },
            Component::Normal(part) => normalized.push(part),
        }
    }

    if normalized.as_os_str().is_empty() {
        normalized.push(".");
    }
    crate::utils::paths::normalize_windows_verbatim_path(&normalized)
}
