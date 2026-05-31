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
fn same_snake_case_type_names_compile_but_generator_rejects_duplicate_key() {
    assert_eq!(IdLocalizer.localize_message(&URLMessage), "url_message");
    assert_eq!(IdLocalizer.localize_message(&UrlMessage), "url_message");

    let infos = es_fluent::registry::get_all_ftl_type_infos()
        .filter(|info| matches!(info.type_name, "URLMessage" | "UrlMessage"))
        .collect::<Vec<_>>();
    assert_eq!(infos.len(), 2);

    let temp = tempfile::tempdir().expect("tempdir");
    let err = es_fluent_generate::generate(
        "derive-name-collision",
        temp.path().join("i18n"),
        temp.path(),
        &infos,
        es_fluent_generate::FluentParseMode::Conservative,
        true,
    )
    .expect_err("generator should reject duplicate generated keys");

    let message = err.to_string();
    assert!(message.contains("Duplicate generated FTL key 'url_message'"));
    assert!(message.contains("URLMessage"));
    assert!(message.contains("UrlMessage"));
}
