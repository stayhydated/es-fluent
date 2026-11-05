use super::CLI_VERSION;
use super::profiles::MemberUnit;
use crate::generation::cache::{MetadataCache, WorkspaceMember};
use anyhow::Result;
use cargo_manifest::{Dependency, DependencyDetail};
use fs_err as fs;
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    path::{Path, PathBuf},
};

type ManifestOverrides = toml::map::Map<String, toml::Value>;

/// Configuration derived from cargo metadata for temp crate generation.
pub(super) struct TempCrateConfig {
    pub(super) es_fluent_dep: Dependency,
    pub(super) es_fluent_cli_helpers_dep: Dependency,
    pub(super) target_dir: PathBuf,
    pub(super) manifest_overrides: ManifestOverrides,
    /// Workspace members by package name.
    pub(super) workspace_members: BTreeMap<String, WorkspaceMember>,
}

impl TempCrateConfig {
    /// Create config by querying cargo metadata once, or from cache if valid.
    pub(super) fn from_manifest(manifest_path: &Path, runner_target_dir: PathBuf) -> Result<Self> {
        let manifest_overrides = Self::extract_manifest_overrides(manifest_path)?;

        let workspace_root = manifest_path.parent().unwrap_or(Path::new("."));
        let temp_dir = es_fluent_runner::RunnerMetadataStore::temp_for_workspace(workspace_root);

        let cache = match MetadataCache::load(temp_dir.base_dir()) {
            Some(cache) if cache.is_valid(workspace_root) => Some(cache),
            _ => Self::query_metadata(manifest_path, workspace_root, temp_dir.base_dir())?,
        };
        let (local_es_fluent, local_helpers, workspace_members) = match cache {
            Some(cache) => (
                cache.local_es_fluent_dep,
                cache.local_es_fluent_cli_helpers_dep,
                cache.workspace_members,
            ),
            None => (None, None, BTreeMap::new()),
        };

        let es_fluent_dep = match local_es_fluent {
            Some(dep) => dep,
            None => Self::find_cli_workspace_dep_es_fluent()?
                .unwrap_or_else(|| Self::version_dep(CLI_VERSION)),
        };
        let es_fluent_cli_helpers_dep = match local_helpers {
            Some(dep) => dep,
            None => Self::find_cli_workspace_dep_helpers()?
                .unwrap_or_else(|| Self::version_dep(CLI_VERSION)),
        };

        Ok(Self {
            es_fluent_dep,
            es_fluent_cli_helpers_dep,
            target_dir: runner_target_dir,
            manifest_overrides,
            workspace_members,
        })
    }

    /// Runs cargo metadata and caches what it reports.
    ///
    /// Returns `None` when cargo metadata fails; nothing is cached then, so the
    /// next run retries it.
    fn query_metadata(
        manifest_path: &Path,
        workspace_root: &Path,
        cache_dir: &Path,
    ) -> Result<Option<MetadataCache>> {
        let Ok(metadata) = cargo_metadata::MetadataCommand::new()
            .manifest_path(manifest_path)
            .no_deps()
            .exec()
        else {
            return Ok(None);
        };

        let manifest_hashes = std::iter::once(manifest_path.to_path_buf())
            .chain(
                metadata
                    .workspace_packages()
                    .into_iter()
                    .map(|package| package.manifest_path.clone().into_std_path_buf()),
            )
            .filter_map(|path| MetadataCache::hash_manifest(&path).map(|hash| (path, hash)))
            .collect();
        let cache = MetadataCache {
            format: MetadataCache::FORMAT,
            cargo_lock_hash: MetadataCache::hash_cargo_lock(workspace_root).unwrap_or_default(),
            local_es_fluent_dep: Self::find_local_dep(&metadata, "es-fluent")?,
            local_es_fluent_cli_helpers_dep: Self::find_local_dep(
                &metadata,
                "es-fluent-cli-helpers",
            )?,
            workspace_members: Self::workspace_members(&metadata),
            manifest_hashes,
        };

        if !cache.cargo_lock_hash.is_empty() {
            let _ = fs::create_dir_all(cache_dir);
            let _ = cache.save(cache_dir);
        }
        Ok(Some(cache))
    }

    /// Records each workspace member's kind and the members it always depends on.
    ///
    /// Optional and platform-specific dependencies are left out: whether the
    /// runner build activates them is unknown here, and a profile override for
    /// a package outside the build graph makes Cargo warn. Members reached only
    /// through them keep the runner's dependency profile.
    fn workspace_members(meta: &cargo_metadata::Metadata) -> BTreeMap<String, WorkspaceMember> {
        let packages = meta.workspace_packages();
        let names = packages
            .iter()
            .map(|package| package.name.as_str())
            .collect::<BTreeSet<_>>();
        packages
            .iter()
            .map(|package| {
                let member_deps = |kind| {
                    package
                        .dependencies
                        .iter()
                        .filter(|dep| {
                            dep.kind == kind
                                && !dep.optional
                                && dep.target.is_none()
                                && dep.path.is_some()
                                && names.contains(dep.name.as_str())
                        })
                        .map(|dep| dep.name.clone())
                        .collect()
                };
                let member = WorkspaceMember {
                    proc_macro: package
                        .targets
                        .iter()
                        .any(|target| target.kind.contains(&cargo_metadata::TargetKind::ProcMacro)),
                    deps: member_deps(cargo_metadata::DependencyKind::Normal),
                    build_deps: member_deps(cargo_metadata::DependencyKind::Build),
                };
                (package.name.to_string(), member)
            })
            .collect()
    }

    /// Classifies the workspace members reachable from the runner's direct
    /// dependencies `roots` by how Cargo compiles them.
    ///
    /// Proc macros, build dependencies, and their dependencies are compiled for
    /// the host. A member reached only that way is [`MemberUnit::HostOnly`].
    pub(super) fn reachable_members<'a>(
        &self,
        roots: impl IntoIterator<Item = &'a str>,
    ) -> BTreeMap<String, MemberUnit> {
        let mut seen = BTreeSet::<(String, bool)>::new();
        let mut pending = roots
            .into_iter()
            .filter_map(|root| {
                let member = self.workspace_members.get(root)?;
                Some((root.to_string(), member.proc_macro))
            })
            .collect::<Vec<_>>();
        while let Some((name, host)) = pending.pop() {
            if !seen.insert((name.clone(), host)) {
                continue;
            }
            let Some(member) = self.workspace_members.get(&name) else {
                continue;
            };
            let next = |dep: &String, host: bool| {
                let proc_macro = self
                    .workspace_members
                    .get(dep)
                    .is_some_and(|member| member.proc_macro);
                (dep.clone(), host || proc_macro)
            };
            pending.extend(member.deps.iter().map(|dep| next(dep, host)));
            pending.extend(member.build_deps.iter().map(|dep| next(dep, true)));
        }

        let mut members = BTreeMap::new();
        for (name, host) in seen {
            let unit = if host {
                MemberUnit::HostOnly
            } else {
                MemberUnit::Target
            };
            members
                .entry(name)
                .and_modify(|existing| {
                    if unit == MemberUnit::Target {
                        *existing = MemberUnit::Target;
                    }
                })
                .or_insert(unit);
        }
        members
    }

    /// Keeps the workspace members reachable from `roots` on their workspace
    /// profile settings; see [`super::profiles`].
    pub(super) fn restore_member_profiles<'a>(&mut self, roots: impl IntoIterator<Item = &'a str>) {
        let members = self.reachable_members(roots);
        if let Some(toml::Value::Table(profiles)) = self.manifest_overrides.get_mut("profile") {
            super::profiles::restore_member_profiles(profiles, &members);
        }
    }

    fn find_local_dep(
        meta: &cargo_metadata::Metadata,
        crate_name: &str,
    ) -> Result<Option<Dependency>> {
        Ok(meta
            .packages
            .iter()
            .find(|p| p.name.as_str() == crate_name && p.source.is_none())
            .map(|pkg| {
                Self::path_dep_utf8(
                    pkg.manifest_path
                        .parent()
                        .expect("manifest path parent")
                        .as_str(),
                )
            }))
    }

    fn find_cli_workspace_dep_es_fluent() -> Result<Option<Dependency>> {
        let cli_manifest_dir = env!("CARGO_MANIFEST_DIR");
        let cli_path = Path::new(cli_manifest_dir);
        let Some(workspace_dir) = cli_path.parent() else {
            return Ok(None);
        };
        let es_fluent_path = workspace_dir.join("es-fluent");
        if es_fluent_path.join("Cargo.toml").exists() {
            Ok(Some(Self::path_dep(
                &es_fluent_path,
                "es-fluent workspace dependency path",
            )?))
        } else {
            Ok(None)
        }
    }

    fn find_cli_workspace_dep_helpers() -> Result<Option<Dependency>> {
        let cli_manifest_dir = env!("CARGO_MANIFEST_DIR");
        let cli_path = Path::new(cli_manifest_dir);
        let Some(workspace_dir) = cli_path.parent() else {
            return Ok(None);
        };
        let helpers_path = workspace_dir.join("es-fluent-cli-helpers");
        if helpers_path.join("Cargo.toml").exists() {
            Ok(Some(Self::path_dep(
                &helpers_path,
                "es-fluent-cli-helpers workspace dependency path",
            )?))
        } else {
            Ok(None)
        }
    }

    /// Extract workspace-root manifest settings the runner crate must mirror.
    ///
    /// The runner crate is an isolated workspace root, so it doesn't inherit the
    /// project's `[patch]` and `[replace]` dependency overrides or its `[profile]`
    /// settings unless they are mirrored into the generated `.es-fluent/Cargo.toml`.
    /// Matching profiles let compiler caches reuse units from the application's own
    /// builds. `cargo-features` is mirrored because profile keys may require it.
    pub(super) fn extract_manifest_overrides(manifest_path: &Path) -> Result<ManifestOverrides> {
        let content = match fs::read_to_string(manifest_path) {
            Ok(content) => content,
            Err(_) => return Ok(ManifestOverrides::new()),
        };

        let parsed: toml::Value = match toml::from_str(&content) {
            Ok(parsed) => parsed,
            Err(_) => return Ok(ManifestOverrides::new()),
        };

        let Some(table) = parsed.as_table() else {
            return Ok(ManifestOverrides::new());
        };

        let mut overrides = ManifestOverrides::new();
        let manifest_dir = manifest_path.parent().unwrap_or(Path::new("."));

        if let Some(patch) = table.get("patch") {
            let mut patch = patch.clone();
            Self::absolutize_override_paths(&mut patch, manifest_dir)?;
            overrides.insert("patch".to_string(), patch);
        }

        if let Some(replace) = table.get("replace") {
            let mut replace = replace.clone();
            Self::absolutize_override_paths(&mut replace, manifest_dir)?;
            overrides.insert("replace".to_string(), replace);
        }

        for key in ["profile", "cargo-features"] {
            if let Some(value) = table.get(key) {
                overrides.insert(key.to_string(), value.clone());
            }
        }

        Ok(overrides)
    }

    fn absolutize_override_paths(value: &mut toml::Value, manifest_dir: &Path) -> Result<()> {
        match value {
            toml::Value::Table(table) => {
                if let Some(toml::Value::String(path)) = table.get_mut("path") {
                    let dependency_path = Path::new(path);
                    if dependency_path.is_relative() {
                        *path = super::utf8_path_string(
                            &manifest_dir.join(dependency_path),
                            "manifest dependency override path",
                        )?;
                    }
                }

                for (_, nested) in table.iter_mut() {
                    Self::absolutize_override_paths(nested, manifest_dir)?;
                }
            },
            toml::Value::Array(array) => {
                for nested in array {
                    Self::absolutize_override_paths(nested, manifest_dir)?;
                }
            },
            _ => {},
        }

        Ok(())
    }

    fn path_dep(path: &Path, context: &str) -> Result<Dependency> {
        let path = super::utf8_path_string(path, context)?;
        Ok(Self::path_dep_utf8(&path))
    }

    fn path_dep_utf8(path: &str) -> Dependency {
        Dependency::Detailed(DependencyDetail {
            path: Some(path.to_string()),
            ..Default::default()
        })
    }

    fn version_dep(version: &str) -> Dependency {
        Dependency::Simple(version.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{ManifestOverrides, MemberUnit, TempCrateConfig};
    use cargo_manifest::Dependency;
    use std::collections::BTreeMap;
    use std::path::Path;
    use toml::Value;

    fn dependency_path(dep: Dependency) -> Option<String> {
        match dep {
            Dependency::Detailed(detail) => detail.path,
            _ => None,
        }
    }

    fn dependency_path_ends_with(dep: Dependency, suffix: &Path) -> bool {
        dependency_path(dep).is_some_and(|path| Path::new(&path).ends_with(suffix))
    }

    fn write_member_workspace(root: &Path, members: &[(&str, &str)]) {
        let names = members
            .iter()
            .map(|(name, _)| format!("\"{name}\""))
            .collect::<Vec<_>>()
            .join(", ");
        std::fs::write(
            root.join("Cargo.toml"),
            format!("[workspace]\nmembers = [{names}]\nresolver = \"2\"\n"),
        )
        .expect("write workspace manifest");
        for (name, extra) in members {
            let dir = root.join(name);
            std::fs::create_dir_all(dir.join("src")).expect("create package");
            std::fs::write(dir.join("src/lib.rs"), "").expect("write lib");
            std::fs::write(
                dir.join("Cargo.toml"),
                format!(
                    "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n{extra}"
                ),
            )
            .expect("write package manifest");
        }
    }

    fn config_for_members(root: &Path) -> TempCrateConfig {
        let metadata = cargo_metadata::MetadataCommand::new()
            .manifest_path(root.join("Cargo.toml"))
            .no_deps()
            .exec()
            .expect("cargo metadata");
        TempCrateConfig {
            es_fluent_dep: TempCrateConfig::version_dep("0.0.0"),
            es_fluent_cli_helpers_dep: TempCrateConfig::version_dep("0.0.0"),
            target_dir: root.join("target"),
            manifest_overrides: ManifestOverrides::new(),
            workspace_members: TempCrateConfig::workspace_members(&metadata),
        }
    }

    #[test]
    fn workspace_members_skip_optional_platform_and_dev_dependencies() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_member_workspace(
            temp.path(),
            &[
                (
                    "app",
                    "[dependencies]\nalways = { path = \"../always\" }\noptional = { path = \"../optional\", optional = true }\n\n[target.'cfg(windows)'.dependencies]\nplatform = { path = \"../platform\" }\n\n[dev-dependencies]\ndev = { path = \"../dev\" }\n",
                ),
                ("always", ""),
                ("optional", ""),
                ("platform", ""),
                ("dev", ""),
            ],
        );

        let members = config_for_members(temp.path()).workspace_members;

        assert_eq!(members["app"].deps, vec!["always".to_string()]);
        assert!(members["app"].build_deps.is_empty());
        assert!(members["always"].deps.is_empty());
    }

    #[test]
    fn reachable_members_mark_proc_macro_and_build_only_members_as_host_only() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_member_workspace(
            temp.path(),
            &[
                (
                    "app",
                    "[dependencies]\nalways = { path = \"../always\" }\npmac = { path = \"../pmac\" }\n\n[build-dependencies]\nbuildtool = { path = \"../buildtool\" }\n",
                ),
                (
                    "always",
                    "[dependencies]\nshared = { path = \"../shared\" }\n",
                ),
                (
                    "pmac",
                    "[lib]\nproc-macro = true\n\n[dependencies]\nhostlib = { path = \"../hostlib\" }\n",
                ),
                (
                    "buildtool",
                    "[dependencies]\nshared = { path = \"../shared\" }\n",
                ),
                ("hostlib", ""),
                ("shared", ""),
                ("unused", ""),
            ],
        );

        let config = config_for_members(temp.path());
        assert!(config.workspace_members["pmac"].proc_macro);
        assert_eq!(
            config.workspace_members["app"].build_deps,
            vec!["buildtool".to_string()]
        );

        let reachable = config.reachable_members(["app", "not-a-member"]);
        assert_eq!(
            reachable,
            BTreeMap::from([
                ("always".to_string(), MemberUnit::Target),
                ("app".to_string(), MemberUnit::Target),
                ("buildtool".to_string(), MemberUnit::HostOnly),
                ("hostlib".to_string(), MemberUnit::HostOnly),
                ("pmac".to_string(), MemberUnit::HostOnly),
                ("shared".to_string(), MemberUnit::Target),
            ])
        );
    }

    #[test]
    fn find_local_dep_returns_path_dependency_for_local_workspace_package() {
        let temp = tempfile::tempdir().expect("tempdir");
        crate::test_fixtures::toml_helpers::write_toml(
            &temp.path().join("Cargo.toml"),
            &crate::test_fixtures::toml_helpers::workspace_manifest(&[
                "app",
                "es-fluent",
                "es-fluent-cli-helpers",
            ]),
        );
        crate::test_fixtures::toml_helpers::write_toml(
            &temp.path().join("app/Cargo.toml"),
            &crate::test_fixtures::toml_helpers::package_manifest("app", "0.1.0"),
        );
        crate::test_fixtures::write_file(&temp.path().join("app/src/lib.rs"), "pub struct App;\n");
        crate::test_fixtures::toml_helpers::write_toml(
            &temp.path().join("es-fluent/Cargo.toml"),
            &crate::test_fixtures::toml_helpers::package_manifest("es-fluent", "0.1.0"),
        );
        crate::test_fixtures::write_file(
            &temp.path().join("es-fluent/src/lib.rs"),
            "pub struct EsFluent;\n",
        );
        crate::test_fixtures::toml_helpers::write_toml(
            &temp.path().join("es-fluent-cli-helpers/Cargo.toml"),
            &crate::test_fixtures::toml_helpers::package_manifest("es-fluent-cli-helpers", "0.1.0"),
        );
        crate::test_fixtures::write_file(
            &temp.path().join("es-fluent-cli-helpers/src/lib.rs"),
            "pub struct Helpers;\n",
        );

        let metadata = cargo_metadata::MetadataCommand::new()
            .manifest_path(temp.path().join("Cargo.toml"))
            .no_deps()
            .exec()
            .expect("load cargo metadata");

        let es_fluent =
            TempCrateConfig::find_local_dep(&metadata, "es-fluent").expect("find local dep");
        assert_eq!(
            dependency_path(es_fluent.expect("expected es-fluent dependency")),
            Some(temp.path().join("es-fluent").display().to_string())
        );

        let helpers = TempCrateConfig::find_local_dep(&metadata, "es-fluent-cli-helpers")
            .expect("find helpers dep");
        assert_eq!(
            dependency_path(helpers.expect("expected helpers dependency")),
            Some(
                temp.path()
                    .join("es-fluent-cli-helpers")
                    .display()
                    .to_string()
            )
        );

        assert!(
            TempCrateConfig::find_local_dep(&metadata, "missing-crate")
                .expect("missing crate lookup")
                .is_none()
        );
    }

    #[test]
    fn find_cli_workspace_deps_use_repo_crate_paths() {
        let es_fluent = TempCrateConfig::find_cli_workspace_dep_es_fluent()
            .expect("resolve es-fluent workspace dep");
        assert!(dependency_path_ends_with(
            es_fluent.expect("expected es-fluent workspace dependency"),
            &Path::new("crates").join("es-fluent"),
        ));

        let helpers = TempCrateConfig::find_cli_workspace_dep_helpers()
            .expect("resolve helpers workspace dep");
        assert!(dependency_path_ends_with(
            helpers.expect("expected helpers workspace dependency"),
            &Path::new("crates").join("es-fluent-cli-helpers"),
        ));
    }

    #[test]
    fn extract_manifest_overrides_rewrites_relative_paths_and_preserves_absolute_paths() {
        let temp = tempfile::tempdir().expect("tempdir");
        let patch_manifest = temp.path().join("patch.toml");
        let mut patch_value: Value = toml::from_str(
            r#"
[patch.crates-io]
serde = { version = "1" }
local-dependency = { path = "../local-dependency" }

[replace]
"replaced-dependency:0.1.0" = { path = "vendor/replaced-dependency" }
"#,
        )
        .expect("parse patch manifest fixture");
        let absolute_dependency = temp.path().join("absolute-dependency");
        patch_value["patch"]["crates-io"]
            .as_table_mut()
            .expect("patch.crates-io fixture table")
            .insert(
                "absolute-dependency".to_string(),
                Value::Table(crate::test_fixtures::toml_helpers::table([(
                    "path",
                    crate::test_fixtures::toml_helpers::string_value(
                        absolute_dependency.to_str().expect("UTF-8 fixture path"),
                    ),
                )])),
            );
        crate::test_fixtures::toml_helpers::write_toml(&patch_manifest, &patch_value);
        let overrides = TempCrateConfig::extract_manifest_overrides(&patch_manifest)
            .expect("extract manifest overrides");
        assert!(overrides.contains_key("patch"));
        assert!(overrides.contains_key("replace"));
        assert_eq!(
            overrides["patch"]["crates-io"]["local-dependency"]["path"].as_str(),
            Some(
                temp.path()
                    .join("../local-dependency")
                    .to_str()
                    .expect("UTF-8 fixture path")
            )
        );
        assert_eq!(
            overrides["patch"]["crates-io"]["absolute-dependency"]["path"].as_str(),
            absolute_dependency.to_str()
        );
        assert_eq!(
            overrides["replace"]["replaced-dependency:0.1.0"]["path"].as_str(),
            temp.path().join("vendor/replaced-dependency").to_str()
        );
    }

    #[test]
    fn extract_manifest_overrides_handles_invalid_inputs() {
        let temp = tempfile::tempdir().expect("tempdir");

        let invalid_manifest = temp.path().join("invalid.toml");
        crate::test_fixtures::write_file(&invalid_manifest, "not = [valid");
        assert!(
            TempCrateConfig::extract_manifest_overrides(&invalid_manifest)
                .expect("ignore invalid manifest")
                .is_empty()
        );

        let non_table_manifest = temp.path().join("scalar.toml");
        crate::test_fixtures::write_file(
            &non_table_manifest,
            &crate::test_fixtures::toml_helpers::string_value("scalar-value").to_string(),
        );
        assert!(
            TempCrateConfig::extract_manifest_overrides(&non_table_manifest)
                .expect("ignore non-table manifest")
                .is_empty()
        );
    }

    #[cfg(unix)]
    #[test]
    fn absolutize_override_paths_reports_non_utf8_rewritten_paths() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt as _;
        use std::path::PathBuf;

        let manifest_dir = PathBuf::from(OsString::from_vec(b"non-utf8-\xff".to_vec()));
        let mut dependency = Value::Table(crate::test_fixtures::toml_helpers::table([(
            "path",
            crate::test_fixtures::toml_helpers::string_value("../local-dependency"),
        )]));

        let error = TempCrateConfig::absolutize_override_paths(&mut dependency, &manifest_dir)
            .expect_err("non-UTF-8 rewritten path should fail");
        assert!(
            error
                .to_string()
                .contains("manifest dependency override path must be valid UTF-8")
        );
    }

    #[test]
    fn dependency_builders_preserve_expected_shapes() {
        let path_dep = TempCrateConfig::path_dep(Path::new("/tmp/example"), "example path")
            .expect("build path dependency");
        assert_eq!(dependency_path(path_dep), Some("/tmp/example".to_string()));

        let utf8_dep = TempCrateConfig::path_dep_utf8("/tmp/example-utf8");
        assert_eq!(
            dependency_path(utf8_dep),
            Some("/tmp/example-utf8".to_string())
        );

        match TempCrateConfig::version_dep("1.2.3") {
            Dependency::Simple(version) => assert_eq!(version, "1.2.3"),
            dep => panic!("expected simple version dependency, got {dep:?}"),
        }
    }
}
