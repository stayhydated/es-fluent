use es_fluent::registry::{__macro, FtlScope, FtlTypeInfo, FtlVariant, RegisteredFtlType};
use es_fluent_shared::meta::TypeKind;

static VARIANTS: &[FtlVariant] = &[FtlVariant::new(
    "Invalid",
    __macro::static_entry_id("invalid"),
    &[],
    "renamed_library",
    1,
)];
static INFO: FtlTypeInfo = FtlTypeInfo::new(
    TypeKind::Struct,
    "InvalidSourcePackage",
    VARIANTS,
    FtlScope::new("invalid package", None),
    "src/lib.rs",
    "renamed_library",
    None,
);

es_fluent::__inventory::submit!(RegisteredFtlType(&INFO));

#[test]
fn malformed_inventory_source_package_is_a_structured_error() {
    let temp = tempfile::tempdir().expect("tempdir");
    let original_dir = std::env::current_dir().expect("current directory");
    std::env::set_current_dir(temp.path()).expect("enter temp directory");

    let result = es_fluent_cli_helpers::write_inventory_for_crate("valid-package");
    std::env::set_current_dir(original_dir).expect("restore current directory");
    let error = result.expect_err("invalid source package should fail");

    assert!(matches!(
        error,
        es_fluent_runner::RunnerIoError::InvalidInventorySourcePackage {
            source_package,
            source_type,
            reason,
        } if source_package == "invalid package"
            && source_type.contains("InvalidSourcePackage")
            && source_type.contains("src/lib.rs")
            && reason.contains("invalid character")
    ));
}
