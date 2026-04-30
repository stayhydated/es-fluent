use super::*;
use es_fluent_manager_core::{ModuleDiscoveryError, ModuleRegistrationKind};

#[test]
fn cached_discovered_modules_returns_cached_discovery_errors() {
    let modules = OnceLock::new();
    modules
        .set(Err(ModuleDiscoveryErrors::from(vec![
            ModuleDiscoveryError::DuplicateModuleRegistration {
                name: "app".to_string(),
                domain: "app".to_string(),
                kind: ModuleRegistrationKind::RuntimeLocalizer,
                count: 2,
            },
        ])))
        .expect("test should set cache once");

    let err = cached_discovered_modules(&modules)
        .expect_err("cached discovery errors should be returned");

    match err {
        DioxusInitError::ModuleDiscovery(errors) => {
            assert_eq!(errors.as_slice().len(), 1);
        },
        other => panic!("expected module discovery error, got {other:?}"),
    }
}
