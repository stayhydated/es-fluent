use super::*;

pub(super) fn validate_container_namespace(
    container_context: &ContainerContext,
    fallback_span: proc_macro2::Span,
) -> Result<(), EsFluentCoreError> {
    validate_namespace(
        container_context
            .fluent_namespace()
            .map(SpannedNamespaceRule::rule),
        container_context
            .fluent_namespace()
            .map(SpannedNamespaceRule::span)
            .unwrap_or(fallback_span),
    )
}

pub(super) fn validate_container_domain(
    container_context: &ContainerContext,
    fallback_span: proc_macro2::Span,
) -> Result<(), EsFluentCoreError> {
    let Some(domain) = container_context.fluent_domain() else {
        return Ok(());
    };

    derive_validation::validate_domain(
        domain,
        Some(
            container_context
                .fluent_domain_with_span()
                .map(crate::semantic::SpannedValue::span)
                .unwrap_or(fallback_span),
        ),
    )
}

pub(super) fn validate_namespace(
    namespace: Option<&NamespaceRule>,
    span: proc_macro2::Span,
) -> Result<(), EsFluentCoreError> {
    if let Some(ns) = namespace
        && let Err(error) = derive_validation::validate_namespace(ns, Some(span))
    {
        return Err(error);
    }

    Ok(())
}
