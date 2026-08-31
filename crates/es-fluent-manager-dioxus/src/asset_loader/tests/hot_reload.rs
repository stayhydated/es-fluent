use super::*;

#[test]
fn dioxus_asset_hot_reload_matching_tracks_bundled_assets() {
    let watched = vec!["i18n/web-123.ftl".to_string(), "other.ftl".to_string()];

    assert!(dioxus_i18n_asset_path_matches(
        "/assets/i18n/web-123.ftl",
        &watched
    ));
    assert!(dioxus_i18n_asset_path_matches(
        "/es-fluent/assets/i18n/web-123.ftl?dx_force_reload=1",
        &watched
    ));
    assert!(!dioxus_i18n_asset_path_matches(
        "/assets/i18n/web-456.ftl",
        &watched
    ));
}

#[test]
fn dioxus_asset_hot_reload_message_matching_reads_devserver_payloads() {
    let watched = vec!["i18n/web-123.ftl".to_string()];
    let matching_message = r#"{
            "HotReload": {
                "templates": [],
                "assets": ["/assets/i18n/web-123.ftl"],
                "ms_elapsed": 0,
                "jump_table": null,
                "for_build_id": null,
                "for_pid": null
            }
        }"#;
    let unrelated_message = r#"{
            "HotReload": {
                "templates": [],
                "assets": ["/assets/i18n/other.ftl"],
                "ms_elapsed": 0,
                "jump_table": null,
                "for_build_id": null,
                "for_pid": null
            }
        }"#;

    assert!(dioxus_i18n_hot_reload_message_matches(
        matching_message,
        &watched
    ));
    assert!(!dioxus_i18n_hot_reload_message_matches(
        unrelated_message,
        &watched
    ));
    assert!(!dioxus_i18n_hot_reload_message_matches(
        r#"{"FullReloadStart": null}"#,
        &watched
    ));
}

#[test]
fn cache_busted_asset_path_appends_query_without_losing_existing_query() {
    assert_eq!(
        cache_busted_asset_path("/assets/web.ftl", 7),
        "/assets/web.ftl?dx_i18n_reload=7"
    );
    assert_eq!(
        cache_busted_asset_path("/assets/web.ftl?existing=1", 8),
        "/assets/web.ftl?existing=1&dx_i18n_reload=8"
    );
}
