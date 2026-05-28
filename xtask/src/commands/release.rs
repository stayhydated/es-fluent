use stayhydated_xtask::release::PublishOptions;

use crate::cli::ReleasePublishArgs;

pub fn plan() -> anyhow::Result<()> {
    let workspace_root = stayhydated_xtask::workspace_root_from_xtask_manifest()?;
    stayhydated_xtask::release::plan(&workspace_root)
}

pub fn publish(args: &ReleasePublishArgs) -> anyhow::Result<()> {
    let workspace_root = stayhydated_xtask::workspace_root_from_xtask_manifest()?;
    stayhydated_xtask::release::publish(&workspace_root, &PublishOptions::from(args))
}

impl From<&ReleasePublishArgs> for PublishOptions {
    fn from(args: &ReleasePublishArgs) -> Self {
        Self::builder()
            .execute(args.execute)
            .maybe_from(args.from.clone())
            .maybe_registry(args.registry.clone())
            .allow_dirty(args.allow_dirty)
            .no_verify(args.no_verify)
            .include_dev_deps(args.include_dev_deps)
            .skip_existing(args.skip_existing)
            .retries(args.retries)
            .retry_delay_seconds(args.retry_delay_seconds)
            .build()
    }
}
