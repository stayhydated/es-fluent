use std::collections::HashSet;

use crate::fluent::FluentDomain;
use crate::namespace::ResolvedNamespace;
use crate::registry::StaticFluentDomain;

use super::{ModuleResourceSpec, ResourceKey, ResourcePlanError};

/// Canonical resource plan for a module domain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourcePlan {
    specs: Vec<ModuleResourceSpec>,
}

impl ResourcePlan {
    /// Builds the global/default canonical resource plan for a validated static domain.
    pub fn for_static_domain(
        domain: StaticFluentDomain,
        namespaces: &[&str],
    ) -> Result<Self, ResourcePlanError> {
        let mut resolved_namespaces = Vec::with_capacity(namespaces.len());
        let mut seen = HashSet::new();

        for namespace in namespaces {
            let namespace = ResolvedNamespace::new(*namespace).map_err(|details| {
                ResourcePlanError::InvalidNamespace {
                    namespace: (*namespace).to_string(),
                    details,
                }
            })?;

            if seen.insert(namespace.clone()) {
                resolved_namespaces.push(namespace);
            }
        }

        Ok(Self::from_static_domain_and_namespaces(
            domain,
            &resolved_namespaces,
            namespaces.is_empty(),
        ))
    }

    fn from_static_domain_and_namespaces(
        domain: StaticFluentDomain,
        namespaces: &[ResolvedNamespace],
        base_required: bool,
    ) -> Self {
        if namespaces.is_empty() {
            return Self {
                specs: vec![ModuleResourceSpec::base_for_static_domain(
                    domain,
                    base_required,
                )],
            };
        }

        let mut specs = Vec::with_capacity(namespaces.len() + 1);
        specs.push(ModuleResourceSpec::base_for_static_domain(domain, false));
        for namespace in namespaces {
            specs.push(ModuleResourceSpec::namespaced_for_static_domain(
                domain, namespace, true,
            ));
        }

        Self { specs }
    }

    /// Builds the global/default canonical resource plan for a domain.
    pub fn for_domain(domain: &str, namespaces: &[&str]) -> Result<Self, ResourcePlanError> {
        let domain =
            FluentDomain::try_new(domain).map_err(|details| ResourcePlanError::InvalidDomain {
                domain: domain.to_string(),
                details,
            })?;

        if namespaces.is_empty() {
            return Ok(Self {
                specs: vec![ModuleResourceSpec::base_for_domain(&domain, true)],
            });
        }

        let mut specs = Vec::with_capacity(namespaces.len() + 1);
        specs.push(ModuleResourceSpec::base_for_domain(&domain, false));

        let mut seen = HashSet::new();
        for namespace in namespaces {
            let namespace = ResolvedNamespace::new(*namespace).map_err(|details| {
                ResourcePlanError::InvalidNamespace {
                    namespace: (*namespace).to_string(),
                    details,
                }
            })?;

            if !seen.insert(namespace.clone()) {
                continue;
            }

            specs.push(ModuleResourceSpec::namespaced_for_domain(
                &domain, &namespace, true,
            ));
        }

        Ok(Self { specs })
    }

    /// Builds a sparse resource plan for a single locale.
    pub fn sparse_for_domain(
        domain: &str,
        has_base_resource: bool,
        namespaces: &[ResolvedNamespace],
        base_required: bool,
    ) -> Self {
        let domain =
            FluentDomain::try_new(domain).expect("sparse resource domain should be validated");
        Self::sparse_for_validated_domain(&domain, has_base_resource, namespaces, base_required)
    }

    /// Builds a sparse resource plan for a single locale from validated static metadata.
    pub fn sparse_for_static_domain(
        domain: StaticFluentDomain,
        has_base_resource: bool,
        namespaces: &[ResolvedNamespace],
        base_required: bool,
    ) -> Self {
        let mut specs = Vec::with_capacity(namespaces.len() + usize::from(has_base_resource));

        if has_base_resource {
            specs.push(ModuleResourceSpec::base_for_static_domain(
                domain,
                base_required,
            ));
        }

        for namespace in namespaces {
            specs.push(ModuleResourceSpec::namespaced_for_static_domain(
                domain, namespace, true,
            ));
        }

        Self { specs }
    }

    fn sparse_for_validated_domain(
        domain: &FluentDomain,
        has_base_resource: bool,
        namespaces: &[ResolvedNamespace],
        base_required: bool,
    ) -> Self {
        let mut specs = Vec::with_capacity(namespaces.len() + usize::from(has_base_resource));

        if has_base_resource {
            specs.push(ModuleResourceSpec::base_for_domain(domain, base_required));
        }

        for namespace in namespaces {
            specs.push(ModuleResourceSpec::namespaced_for_domain(
                domain, namespace, true,
            ));
        }

        Self { specs }
    }

    /// Returns the resource specs in this plan.
    pub fn specs(&self) -> &[ModuleResourceSpec] {
        &self.specs
    }

    /// Converts the plan into its resource specs.
    pub fn into_specs(self) -> Vec<ModuleResourceSpec> {
        self.specs
    }
}

impl From<ResourcePlan> for Vec<ModuleResourceSpec> {
    fn from(plan: ResourcePlan) -> Self {
        plan.into_specs()
    }
}

/// Builds the global/default canonical resource plan for a domain.
///
/// Contract:
/// - Without namespaces, `{domain}.ftl` is required.
/// - With namespaces, `{domain}.ftl` remains an optional mixed-mode resource
///   and `{domain}/{namespace}.ftl` entries are required.
/// - Compile-time registrations may provide a sparse per-language plan through
///   `I18nModuleRegistration::resource_plan_for_language`; managers should use
///   that plan directly when it is available.
pub fn try_resource_plan_for(
    domain: &str,
    namespaces: &[&str],
) -> Result<Vec<ModuleResourceSpec>, ResourcePlanError> {
    ResourcePlan::for_domain(domain, namespaces).map(ResourcePlan::into_specs)
}

/// Builds the global/default canonical resource plan for a domain.
///
/// Use [`try_resource_plan_for`] for dynamic namespace input that should return
/// typed errors. This constructor is for static module metadata paths where
/// invalid namespaces are programmer errors and recovery is not useful.
#[allow(
    clippy::panic,
    reason = "static metadata uses literal namespaces; use try_resource_plan_for for dynamic input"
)]
pub fn resource_plan_for(domain: &str, namespaces: &[&str]) -> Vec<ModuleResourceSpec> {
    try_resource_plan_for(domain, namespaces)
        .unwrap_or_else(|error| panic!("resource_plan_for received {error}"))
}

/// Returns required resource keys from a resource plan.
pub fn required_resource_keys_from_plan(plan: &[ModuleResourceSpec]) -> HashSet<ResourceKey> {
    plan.iter()
        .filter(|spec| spec.required)
        .map(|spec| spec.key.clone())
        .collect()
}

/// Returns optional resource keys from a resource plan.
pub fn optional_resource_keys_from_plan(plan: &[ModuleResourceSpec]) -> HashSet<ResourceKey> {
    plan.iter()
        .filter(|spec| !spec.required)
        .map(|spec| spec.key.clone())
        .collect()
}

/// Returns true when all required keys are present in the loaded set.
pub fn locale_is_ready(
    required_keys: &HashSet<ResourceKey>,
    loaded_keys: &HashSet<ResourceKey>,
) -> bool {
    required_keys.iter().all(|key| loaded_keys.contains(key))
}
