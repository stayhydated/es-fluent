#![cfg(feature = "derive")]

use es_fluent::{EsFluent, FluentLocalizer, FluentLocalizerExt as _, FluentValue};
use std::collections::HashMap;

#[derive(EsFluent)]
struct URLMessage;

#[derive(EsFluent)]
struct UrlMessage;

struct IdLocalizer;

impl FluentLocalizer for IdLocalizer {
    fn localize<'a>(
        &self,
        id: &str,
        _args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        Some(id.to_string())
    }

    fn localize_in_domain<'a>(
        &self,
        _domain: &str,
        id: &str,
        _args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        Some(id.to_string())
    }
}

#[test]
fn same_snake_case_type_names_compile() {
    assert_eq!(IdLocalizer.localize_message(&URLMessage), "url_message");
    assert_eq!(IdLocalizer.localize_message(&UrlMessage), "url_message");
}
