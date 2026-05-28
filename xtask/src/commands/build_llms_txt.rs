const BASE_URL: &str = "https://stayhydated.github.io/es-fluent";
const LLMS_HEADER: &str = include_str!("../../templates/llms-header.md");

pub fn run() -> anyhow::Result<()> {
    let workspace_root = stayhydated_xtask::workspace_root_from_xtask_manifest()?;
    stayhydated_xtask::llms::build_workspace_llms(&workspace_root, BASE_URL, LLMS_HEADER, None)
}
