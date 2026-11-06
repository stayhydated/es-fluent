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
    BUILD_MANIFEST_ENV, BUILD_MANIFEST_FILE_NAME, BUILD_MANIFEST_HASH_ENV, FallbackCatalog,
    FallbackCatalogError, FallbackCatalogIndex, FluentCatalogEntry, FluentCatalogEntryKind,
    INVENTORY_RUNNER_ENV, classify_fluent_entry,
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
