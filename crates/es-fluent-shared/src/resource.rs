//! Shared resource planning types used by managers and generation tooling.

mod catalog;
mod discovery;
mod errors;
mod keys;
mod paths;
mod plan;
mod route;
mod spec;

pub use catalog::{
    FALLBACK_CATALOG_ENV, FALLBACK_CATALOG_FILE_NAME, FallbackCatalog, FallbackCatalogError,
    FluentCatalogEntry, FluentCatalogEntryKind, INVENTORY_RUNNER_ENV, classify_fluent_entry,
    fallback_catalog_contains,
};
pub use discovery::SparseAssetResourcePlans;
pub use errors::{ResourcePlanError, SparseAssetResourcePlanError};
pub use keys::{ResourceKey, ResourceKeyError};
pub use paths::{LocaleRelativeFtlPath, LocaleRelativeFtlPathError};
pub use plan::{
    ResourcePlan, locale_is_ready, optional_resource_keys_from_plan,
    required_resource_keys_from_plan, resource_plan_for, try_resource_plan_for,
};
pub use route::ResourceRoute;
pub use spec::ModuleResourceSpec;

#[cfg(test)]
mod tests;
