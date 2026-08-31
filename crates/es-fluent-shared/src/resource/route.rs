use crate::fluent::FluentDomain;
use crate::namespace::ResolvedNamespace;
use crate::registry::StaticFluentDomain;

use super::ModuleResourceSpec;

/// Canonical route for a crate resource before it is expanded for a locale.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ResourceRoute {
    /// The base crate resource, written as `{domain}.ftl`.
    Base,
    /// A namespaced crate resource, written under `{domain}/{namespace}.ftl`.
    Namespaced(ResolvedNamespace),
}

impl ResourceRoute {
    /// Creates a route from an optional resolved namespace.
    pub fn from_namespace(namespace: Option<ResolvedNamespace>) -> Self {
        match namespace {
            Some(namespace) => Self::Namespaced(namespace),
            None => Self::Base,
        }
    }

    /// Returns true when this route points at the base crate resource.
    pub fn is_base(&self) -> bool {
        matches!(self, Self::Base)
    }

    /// Builds the concrete module resource specification for a crate domain.
    pub fn resource_spec(&self, domain: &str, required: bool) -> ModuleResourceSpec {
        let domain =
            FluentDomain::try_new(domain).expect("resource route domain should be validated");
        self.resource_spec_for_domain(&domain, required)
    }

    /// Builds the concrete module resource specification for a validated crate domain.
    pub fn resource_spec_for_static_domain(
        &self,
        domain: StaticFluentDomain,
        required: bool,
    ) -> ModuleResourceSpec {
        match self {
            Self::Base => ModuleResourceSpec::base_for_static_domain(domain, required),
            Self::Namespaced(namespace) => {
                ModuleResourceSpec::namespaced_for_static_domain(domain, namespace, required)
            },
        }
    }

    fn resource_spec_for_domain(
        &self,
        domain: &FluentDomain,
        required: bool,
    ) -> ModuleResourceSpec {
        match self {
            Self::Base => ModuleResourceSpec::base_for_domain(domain, required),
            Self::Namespaced(namespace) => {
                ModuleResourceSpec::namespaced_for_domain(domain, namespace, required)
            },
        }
    }
}
