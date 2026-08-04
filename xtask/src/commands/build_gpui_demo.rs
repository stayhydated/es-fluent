use stayhydated_xtask::trunk::{TrunkDemoBuildConfig, TrunkDemoPageConfig};

const REQUIRED_MARKER: &str = "GpuiScreenMessages";

pub fn run() -> anyhow::Result<()> {
    let workspace_root = stayhydated_xtask::workspace_root_from_xtask_manifest()?;

    stayhydated_xtask::trunk::build(
        &TrunkDemoBuildConfig::builder()
            .workspace_root(workspace_root)
            .example_dir("examples/gpui-example")
            .output_dir("web/public/gpui-demo")
            .example_name("gpui-example")
            .required_marker(REQUIRED_MARKER)
            .toolchain("nightly")
            .generated_page(
                TrunkDemoPageConfig::builder()
                    .title("es-fluent GPUI demo")
                    .demo_name("GPUI")
                    .build(),
            )
            .build(),
    )
}
