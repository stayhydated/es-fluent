use es_fluent_core::error::{ErrorExt, EsFluentCoreError};
use proc_macro2::Span;

#[test]
fn attribute_error_message_snapshot() {
    let err = EsFluentCoreError::AttributeError {
        message: "Unknown attribute 'foo'".to_string(),
        span: None,
    };

    insta::assert_ron_snapshot!(
        "error_messages__attribute_error_message_snapshot",
        err.to_string()
    );
}

#[test]
fn variant_error_with_note_and_help_snapshot() {
    let err = EsFluentCoreError::VariantError {
        message: "Unsupported variant style".to_string(),
        variant_name: "Data".to_string(),
        span: Some(Span::call_site()),
    }
    .with_note("Only unit or tuple variants are supported here".to_string())
    .with_help("Consider splitting 'Data' into unit + struct variants as needed".to_string());

    insta::assert_ron_snapshot!(
        "error_messages__variant_error_with_note_and_help_snapshot",
        err.to_string()
    );
}

#[test]
fn field_error_named_and_unnamed_snapshot() {
    let named = EsFluentCoreError::FieldError {
        message: "Cannot be both #[fluent(skip)] and #[fluent(default)]".to_string(),
        field_name: Some("value".to_string()),
        span: None,
    };

    let unnamed = EsFluentCoreError::FieldError {
        message: "Unexpected attribute on tuple field".to_string(),
        field_name: None,
        span: None,
    };

    let cases = vec![
        ("named", named.to_string()),
        ("unnamed", unnamed.to_string()),
    ];

    insta::assert_ron_snapshot!(
        "error_messages__field_error_named_and_unnamed_snapshot",
        &cases
    );
}

#[test]
fn transform_error_message_snapshot() {
    let err = EsFluentCoreError::TransformError {
        message: "Failed to convert type: Path<'a> -> String".to_string(),
        span: None,
    }
    .with_help("Implement TryFrom<Path<'a>> for String or adjust your types".to_string());

    insta::assert_ron_snapshot!(
        "error_messages__transform_error_message_snapshot",
        err.to_string()
    );
}
