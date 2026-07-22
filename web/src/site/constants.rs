use stayhydated_dioxus::Project;

pub(crate) const PROJECT: Project = Project::EsFluent;
pub(crate) const SITE_URL: &str = PROJECT.site_url();
pub(crate) const VERSION: &str = env!("CARGO_PKG_VERSION");
