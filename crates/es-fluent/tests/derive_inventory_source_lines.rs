#![cfg(feature = "derive")]

use es_fluent::{EsFluent, EsFluentLabel, EsFluentVariants};

#[derive(EsFluent)]
#[allow(dead_code)]
enum InventoryLineEnum {
    First,                      // SOURCE_LINE_ENUM_FIRST
    WithArgs { value: String }, // SOURCE_LINE_ENUM_WITH_ARGS
}

#[derive(EsFluent)]
#[allow(dead_code)]
struct InventoryLineStruct {
    value: String,
}

#[derive(EsFluentVariants)]
#[fluent_variants(keys = ["label"])]
#[allow(dead_code)]
struct InventoryLineVariantFields {
    first_name: String, // SOURCE_LINE_FIELD_FIRST_NAME
    last_name: String,
}

#[derive(EsFluentLabel)]
#[allow(dead_code)]
enum InventoryLineLabel {
    A,
}

fn marker_line(marker: &str) -> u32 {
    include_str!("derive_inventory_source_lines.rs")
        .lines()
        .position(|line| line.contains(marker))
        .map(|index| index as u32 + 1)
        .expect("marker exists")
}

fn inventory_line(type_name: &str, ftl_key: &str) -> u32 {
    let variant = es_fluent::registry::get_all_ftl_type_infos()
        .filter(|info| info.type_name() == type_name)
        .flat_map(|info| info.variants())
        .find(|variant| variant.entry_id().as_str() == ftl_key)
        .unwrap_or_else(|| panic!("registered key {ftl_key} exists"));

    variant.source_line().get()
}

#[test]
fn derive_inventory_source_lines_point_at_source_items() {
    assert_eq!(
        inventory_line("InventoryLineEnum", "inventory_line_enum-First"),
        marker_line("SOURCE_LINE_ENUM_FIRST")
    );
    assert_eq!(
        inventory_line("InventoryLineEnum", "inventory_line_enum-WithArgs"),
        marker_line("SOURCE_LINE_ENUM_WITH_ARGS")
    );
    assert_eq!(
        inventory_line("InventoryLineStruct", "inventory_line_struct"),
        marker_line("struct InventoryLineStruct")
    );
    assert_eq!(
        inventory_line(
            "InventoryLineVariantFieldsLabelVariants",
            "inventory_line_variant_fields_label_variants-first_name"
        ),
        marker_line("SOURCE_LINE_FIELD_FIRST_NAME")
    );
    assert_eq!(
        inventory_line("InventoryLineLabel", "inventory_line_label_label"),
        marker_line("enum InventoryLineLabel")
    );
}
