use std::{
    collections::{HashMap, HashSet},
    io::{self, Write},
    path::Path,
    process::{Command, Output},
    thread,
    time::Duration,
};

use anyhow::{Context, bail};
use cargo_metadata::{DependencyKind, MetadataCommand, Package, PackageId};

use crate::cli::ReleasePublishArgs;

#[derive(Clone, Debug)]
struct ReleasePackage {
    id: PackageId,
    name: String,
    version: String,
}

pub fn plan() -> anyhow::Result<()> {
    let workspace_root = crate::util::workspace_root()?;
    let packages = release_order(&workspace_root)?;
    print_order(&packages);
    Ok(())
}

pub fn publish(args: &ReleasePublishArgs) -> anyhow::Result<()> {
    let workspace_root = crate::util::workspace_root()?;
    let packages = release_order(&workspace_root)?;
    let packages = packages_from(&packages, args.from.as_deref())?;

    print_order(packages);

    if !args.execute {
        println!();
        println!("No packages were uploaded. Add --execute to run:");
        for package in packages {
            println!("  {}", cargo_publish_command(package, args).join(" "));
        }
        return Ok(());
    }

    if !args.include_dev_deps {
        ensure_cargo_hack()?;
    }

    for package in packages {
        publish_package(&workspace_root, package, args)?;
    }

    Ok(())
}

fn release_order(workspace_root: &Path) -> anyhow::Result<Vec<ReleasePackage>> {
    let metadata = MetadataCommand::new()
        .manifest_path(workspace_root.join("Cargo.toml"))
        .exec()
        .context("failed to read cargo metadata")?;

    let package_by_id = metadata
        .packages
        .iter()
        .map(|package| (package.id.clone(), package))
        .collect::<HashMap<_, _>>();

    let publishable = metadata
        .workspace_members
        .iter()
        .filter_map(|id| package_by_id.get(id).copied())
        .filter(|package| is_publishable(package))
        .collect::<Vec<_>>();

    let publishable_ids = publishable
        .iter()
        .map(|package| package.id.clone())
        .collect::<HashSet<_>>();
    let package_name_to_id = publishable
        .iter()
        .map(|package| (package.name.to_string(), package.id.clone()))
        .collect::<HashMap<_, _>>();
    let workspace_index = publishable
        .iter()
        .enumerate()
        .map(|(index, package)| (package.id.clone(), index))
        .collect::<HashMap<_, _>>();

    let mut remaining_deps = publishable
        .iter()
        .map(|package| {
            let deps = package
                .dependencies
                .iter()
                .filter(|dependency| !matches!(dependency.kind, DependencyKind::Development))
                .filter_map(|dependency| package_name_to_id.get(&dependency.name.to_string()))
                .filter(|dependency_id| publishable_ids.contains(*dependency_id))
                .cloned()
                .collect::<HashSet<_>>();
            (package.id.clone(), deps)
        })
        .collect::<HashMap<_, _>>();

    let mut dependents = HashMap::<PackageId, Vec<PackageId>>::new();
    for (package_id, deps) in &remaining_deps {
        for dep_id in deps {
            dependents
                .entry(dep_id.clone())
                .or_default()
                .push(package_id.clone());
        }
    }

    let mut ready = remaining_deps
        .iter()
        .filter_map(|(package_id, deps)| {
            if deps.is_empty() {
                Some(package_id.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    sort_by_workspace_index(&mut ready, &workspace_index);

    let mut ordered = Vec::new();
    while let Some(package_id) = ready.first().cloned() {
        ready.remove(0);

        let package = package_by_id
            .get(&package_id)
            .with_context(|| format!("metadata missing package {package_id}"))?;
        ordered.push(ReleasePackage {
            id: package.id.clone(),
            name: package.name.to_string(),
            version: package.version.to_string(),
        });

        for dependent_id in dependents.get(&package_id).into_iter().flatten() {
            let deps = remaining_deps
                .get_mut(dependent_id)
                .with_context(|| format!("metadata missing dependent package {dependent_id}"))?;
            deps.remove(&package_id);
            if deps.is_empty() && !ordered.iter().any(|package| package.id == *dependent_id) {
                ready.push(dependent_id.clone());
            }
        }
        sort_by_workspace_index(&mut ready, &workspace_index);
    }

    if ordered.len() != publishable.len() {
        let ordered_ids = ordered
            .iter()
            .map(|package| package.id.clone())
            .collect::<HashSet<_>>();
        let blocked = publishable
            .iter()
            .filter(|package| !ordered_ids.contains(&package.id))
            .map(|package| {
                let deps = remaining_deps
                    .get(&package.id)
                    .into_iter()
                    .flat_map(|deps| deps.iter())
                    .filter_map(|dep_id| package_by_id.get(dep_id))
                    .map(|dep| dep.name.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{} waits on {}", package.name, deps)
            })
            .collect::<Vec<_>>();
        bail!(
            "workspace publish dependencies contain a cycle: {}",
            blocked.join("; ")
        );
    }

    Ok(ordered)
}

fn packages_from<'a>(
    packages: &'a [ReleasePackage],
    from: Option<&str>,
) -> anyhow::Result<&'a [ReleasePackage]> {
    let Some(from) = from else {
        return Ok(packages);
    };

    let index = packages
        .iter()
        .position(|package| package.name == from)
        .with_context(|| {
            let names = packages
                .iter()
                .map(|package| package.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            format!("unknown release package `{from}`; expected one of: {names}")
        })?;

    Ok(&packages[index..])
}

fn publish_package(
    workspace_root: &Path,
    package: &ReleasePackage,
    args: &ReleasePublishArgs,
) -> anyhow::Result<()> {
    let command = cargo_publish_command(package, args);
    for attempt in 0..=args.retries {
        println!();
        println!("Running {}", command.join(" "));

        let output = Command::new(&command[0])
            .current_dir(workspace_root)
            .args(&command[1..])
            .output()
            .with_context(|| format!("failed to run {}", command.join(" ")))?;

        print_output(&output)?;

        if output.status.success() {
            return Ok(());
        }

        if args.skip_existing && output_mentions_existing_upload(&output) {
            println!(
                "{} {} is already uploaded; continuing because --skip-existing was set",
                package.name, package.version
            );
            return Ok(());
        }

        if attempt == args.retries {
            bail!(
                "{} failed after {} attempt(s) with status {}",
                command.join(" "),
                attempt + 1,
                output.status
            );
        }

        println!(
            "Publish failed; retrying in {}s for crates.io index propagation",
            args.retry_delay_seconds
        );
        thread::sleep(Duration::from_secs(args.retry_delay_seconds));
    }

    Ok(())
}

fn ensure_cargo_hack() -> anyhow::Result<()> {
    let output = Command::new("cargo")
        .args(["hack", "--version"])
        .output()
        .context("failed to run `cargo hack --version`")?;

    if output.status.success() {
        return Ok(());
    }

    print_output(&output)?;
    bail!(
        "release publish requires cargo-hack; install it with `cargo install cargo-hack` or pass --include-dev-deps"
    );
}

fn cargo_publish_command(package: &ReleasePackage, args: &ReleasePublishArgs) -> Vec<String> {
    let mut command = if args.include_dev_deps {
        vec![
            "cargo".to_owned(),
            "publish".to_owned(),
            "-p".to_owned(),
            package.name.clone(),
        ]
    } else {
        vec![
            "cargo".to_owned(),
            "hack".to_owned(),
            "--no-dev-deps".to_owned(),
            "publish".to_owned(),
            "-p".to_owned(),
            package.name.clone(),
        ]
    };

    if let Some(registry) = &args.registry {
        command.push("--registry".to_owned());
        command.push(registry.clone());
    }
    if args.allow_dirty {
        command.push("--allow-dirty".to_owned());
    }
    if args.no_verify {
        command.push("--no-verify".to_owned());
    }

    command
}

fn print_order(packages: &[ReleasePackage]) {
    println!("Release publish order:");
    for (index, package) in packages.iter().enumerate() {
        println!("{:>2}. {} {}", index + 1, package.name, package.version);
    }
    println!("Order is computed from non-dev workspace dependencies.");
}

fn print_output(output: &Output) -> anyhow::Result<()> {
    io::stdout().write_all(&output.stdout)?;
    io::stderr().write_all(&output.stderr)?;
    Ok(())
}

fn output_mentions_existing_upload(output: &Output) -> bool {
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    stderr.contains("already uploaded") || stderr.contains("already exists")
}

fn sort_by_workspace_index(
    package_ids: &mut [PackageId],
    workspace_index: &HashMap<PackageId, usize>,
) {
    package_ids.sort_by_key(|package_id| workspace_index.get(package_id).copied());
}

fn is_publishable(package: &Package) -> bool {
    package
        .publish
        .as_ref()
        .is_none_or(|registries| !registries.is_empty())
}
