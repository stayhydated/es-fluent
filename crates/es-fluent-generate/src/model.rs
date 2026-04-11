use crate::formatting;
use es_fluent_derive_core::namer::FluentKey;
use es_fluent_derive_core::registry::{FtlTypeInfo, FtlVariant};

/// Internal owned variant model used during merge and generation.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct OwnedVariant {
    pub(crate) name: String,
    pub(crate) ftl_key: String,
    pub(crate) args: Vec<String>,
}

impl From<&FtlVariant> for OwnedVariant {
    fn from(v: &FtlVariant) -> Self {
        Self {
            name: v.name.to_string(),
            ftl_key: v.ftl_key.to_string(),
            args: v.args.iter().map(|s| s.to_string()).collect(),
        }
    }
}

/// Internal owned type info model used during merge and generation.
#[derive(Clone, Debug)]
pub(crate) struct OwnedTypeInfo {
    pub(crate) type_name: String,
    pub(crate) variants: Vec<OwnedVariant>,
}

impl From<&FtlTypeInfo> for OwnedTypeInfo {
    fn from(info: &FtlTypeInfo) -> Self {
        Self {
            type_name: info.type_name.to_string(),
            variants: info.variants.iter().map(OwnedVariant::from).collect(),
        }
    }
}

/// Compare two type infos, putting `this` entries first.
pub(crate) fn compare_type_infos(a: &OwnedTypeInfo, b: &OwnedTypeInfo) -> std::cmp::Ordering {
    let a_is_this = a
        .variants
        .iter()
        .any(|v| v.ftl_key.ends_with(FluentKey::THIS_SUFFIX));
    let b_is_this = b
        .variants
        .iter()
        .any(|v| v.ftl_key.ends_with(FluentKey::THIS_SUFFIX));

    formatting::compare_with_this_priority(a_is_this, &a.type_name, b_is_this, &b.type_name)
}

/// Merge duplicate `FtlTypeInfo` entries into a stable owned representation.
pub(crate) fn merge_ftl_type_infos(items: &[&FtlTypeInfo]) -> Vec<OwnedTypeInfo> {
    use std::collections::BTreeMap;

    let mut grouped: BTreeMap<String, Vec<OwnedVariant>> = BTreeMap::new();

    for item in items {
        let entry = grouped.entry(item.type_name.to_string()).or_default();
        entry.extend(item.variants.iter().map(OwnedVariant::from));
    }

    grouped
        .into_iter()
        .map(|(type_name, mut variants)| {
            variants.sort_by(|a, b| {
                let a_is_this = a.ftl_key.ends_with(FluentKey::THIS_SUFFIX);
                let b_is_this = b.ftl_key.ends_with(FluentKey::THIS_SUFFIX);
                formatting::compare_with_this_priority(a_is_this, &a.name, b_is_this, &b.name)
            });
            variants.dedup();

            OwnedTypeInfo {
                type_name,
                variants,
            }
        })
        .collect()
}
