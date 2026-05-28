use stayhydated_xtask::web::WebBuildConfig;

pub fn run() -> anyhow::Result<()> {
    let workspace_root = stayhydated_xtask::workspace_root_from_xtask_manifest()?;
    stayhydated_xtask::web::build(
        WebBuildConfig::github_pages(&workspace_root)
            .command_current_dir(workspace_root.join("web"))
            .no_public_assets_dir()
            .extra_dir("web/assets", "assets")
            .extra_file("web/public/assets/site.css", "assets/site.css")
            .extra_dir("web/public/bevy-demo", "bevy-demo")
            .extra_dir("web/public/gpui-demo", "gpui-demo")
            .sitemap_xml(web::sitemap_xml())
            .build(),
    )
}
