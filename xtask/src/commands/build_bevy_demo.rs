use stayhydated_xtask::trunk::{TrunkDemoBuildConfig, TrunkDemoCopyDir, TrunkDemoPageConfig};

// The `#[iso_locale]` autonym of example-shared-lib's `fr-FR` locale proves that
// the demo's language labels survive release optimization.
const REQUIRED_MARKER: &str = "français (France)";

pub fn run() -> anyhow::Result<()> {
    let workspace_root = stayhydated_xtask::workspace_root_from_xtask_manifest()?;

    stayhydated_xtask::trunk::build(
        &TrunkDemoBuildConfig::builder()
            .workspace_root(workspace_root)
            .example_dir("examples/bevy-example")
            .output_dir("web/public/bevy-demo")
            .example_name("bevy-example")
            .required_marker(REQUIRED_MARKER)
            .generated_page(
                TrunkDemoPageConfig::builder()
                    .title("es-fluent Bevy demo")
                    .demo_name("Bevy")
                    .copy_dirs(vec![TrunkDemoCopyDir::new("assets", "assets")])
                    .build(),
            )
            .build(),
    )
}
