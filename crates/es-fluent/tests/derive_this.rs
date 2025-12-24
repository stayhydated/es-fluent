use es_fluent::{EsFluent, EsFluentKv, EsFluentThis, ThisFtl};

#[derive(EsFluent, EsFluentThis)]
#[fluent_this(origin)]
struct TestStruct {
    field: String,
}

#[derive(EsFluentKv, EsFluentThis)]
#[fluent_kv(keys = ["label"])]
#[fluent_this(members)]
struct TestKvStruct {
    field: String,
}

#[derive(EsFluentKv, EsFluentThis)]
#[fluent_kv(keys = ["description"])]
#[fluent_this(members)]
enum TestKvEnum {
    VariantA,
}

#[test]
fn test_derive_this_struct() {
    // Basic ThisFtl on struct
    assert_eq!(TestStruct::this_ftl(), "test_struct_this");
}

#[test]
fn test_derive_this_fields() {
    // ThisFtl on generated KV enum for struct fields
    // Generated name: TestKvStruct + Label + KvFtl = TestKvStructLabelKvFtl
    assert_eq!(
        TestKvStructLabelKvFtl::this_ftl(),
        "test_kv_struct_label_kv_ftl_this"
    );
}

#[test]
fn test_derive_this_variants() {
    // ThisFtl on generated KV enum for enum variants
    // Generated name: TestKvEnum + Description + KvFtl = TestKvEnumDescriptionKvFtl
    assert_eq!(
        TestKvEnumDescriptionKvFtl::this_ftl(),
        "test_kv_enum_description_kv_ftl_this"
    );
}
