use stayhydated_xtask::release::PublishOptions;

use crate::cli::ReleasePublishArgs;

pub fn plan() -> anyhow::Result<()> {
    let workspace_root = stayhydated_xtask::workspace_root_from_xtask_manifest()?;
    stayhydated_xtask::release::plan(&workspace_root)
}

pub fn publish(args: &ReleasePublishArgs) -> anyhow::Result<()> {
    let workspace_root = stayhydated_xtask::workspace_root_from_xtask_manifest()?;
    let options = publish_options(args)?;
    stayhydated_xtask::release::publish(&workspace_root, &options)
}

fn publish_options(args: &ReleasePublishArgs) -> anyhow::Result<PublishOptions> {
    PublishOptions::new(args.execute)
        .resume_from(args.from.clone())?
        .registry(args.registry.clone())
        .map(|options| {
            options
                .allow_dirty(args.allow_dirty)
                .no_verify(args.no_verify)
                .include_dev_deps(args.include_dev_deps)
                .skip_existing(args.skip_existing)
                .retries(args.retries)
                .retry_delay_seconds(args.retry_delay_seconds)
        })
}
