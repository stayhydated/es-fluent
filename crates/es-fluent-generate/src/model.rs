use crate::formatting;
use es_fluent_shared::fluent::{FluentArgumentName, FluentEntryId};
use es_fluent_shared::namer::FluentKey;
use es_fluent_shared::registry::{FtlTypeInfo, FtlVariant};
use es_fluent_shared::{EsFluentError, EsFluentResult};

/// Internal owned variant model used during merge and generation.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct OwnedVariant {
    pub(crate) name: String,
    pub(crate) ftl_key: FluentEntryId,
    pub(crate) args: Vec<FluentArgumentName>,
}

impl OwnedVariant {
    #[cfg(test)]
    pub(crate) fn new(
        name: impl Into<String>,
        ftl_key: impl Into<String>,
        args: impl IntoIterator<Item = impl Into<String>>,
    ) -> EsFluentResult<Self> {
        let ftl_key = ftl_key.into();
        let entry_id = FluentEntryId::try_new(ftl_key.clone()).map_err(|err| {
            EsFluentError::invalid_fluent_identifier(ftl_key.clone(), err.to_string())
        })?;
        let args = args
            .into_iter()
            .map(|arg| {
                let arg = arg.into();
                FluentArgumentName::try_new(arg.clone())
                    .map_err(|err| EsFluentError::invalid_fluent_identifier(arg, err.to_string()))
            })
            .collect::<EsFluentResult<Vec<_>>>()?;

        Ok(Self {
            name: name.into(),
            ftl_key: entry_id,
            args,
        })
    }

    pub(crate) fn from_ftl_variant(variant: &FtlVariant) -> EsFluentResult<Self> {
        Ok(Self {
            name: variant.name.to_string(),
            ftl_key: variant.entry_id(),
            args: variant.argument_names(),
        })
    }

    pub(crate) fn ftl_key(&self) -> &str {
        self.ftl_key.as_str()
    }

    pub(crate) fn is_label(&self) -> bool {
        self.ftl_key().ends_with(FluentKey::LABEL_SUFFIX)
    }
}

/// Internal owned type info model used during merge and generation.
#[derive(Clone, Debug)]
pub(crate) struct OwnedTypeInfo {
    pub(crate) type_name: String,
    pub(crate) variants: Vec<OwnedVariant>,
}

impl OwnedTypeInfo {
    pub(crate) fn from_ftl_type_info(info: &FtlTypeInfo) -> EsFluentResult<Self> {
        Ok(Self {
            type_name: info.type_name.to_string(),
            variants: info
                .variants
                .iter()
                .map(OwnedVariant::from_ftl_variant)
                .collect::<EsFluentResult<Vec<_>>>()?,
        })
    }
}

/// Compare two type infos, putting label entries first.
pub(crate) fn compare_type_infos(a: &OwnedTypeInfo, b: &OwnedTypeInfo) -> std::cmp::Ordering {
    let a_is_label = a.variants.iter().any(OwnedVariant::is_label);
    let b_is_label = b.variants.iter().any(OwnedVariant::is_label);

    formatting::compare_with_label_priority(a_is_label, &a.type_name, b_is_label, &b.type_name)
}

pub(crate) fn validate_no_duplicate_ftl_keys(items: &[&FtlTypeInfo]) -> EsFluentResult<()> {
    use std::collections::BTreeMap;

    let mut seen: BTreeMap<FluentEntryId, (&FtlTypeInfo, &FtlVariant)> = BTreeMap::new();

    for info in items {
        for variant in info.variants {
            let key = variant.entry_id();
            if let Some((first_info, first_variant)) = seen.get(&key) {
                return Err(EsFluentError::duplicate_generated_ftl_key(
                    key.as_str(),
                    first_info.source_description_for(first_variant),
                    info.source_description_for(variant),
                ));
            }

            seen.insert(key, (*info, variant));
        }
    }

    Ok(())
}

/// Merge duplicate `FtlTypeInfo` entries into a stable owned representation.
pub(crate) fn merge_ftl_type_infos(items: &[&FtlTypeInfo]) -> EsFluentResult<Vec<OwnedTypeInfo>> {
    use std::collections::BTreeMap;

    validate_no_duplicate_ftl_keys(items)?;

    let mut grouped: BTreeMap<String, Vec<OwnedVariant>> = BTreeMap::new();

    for item in items {
        let owned = OwnedTypeInfo::from_ftl_type_info(item)?;
        grouped
            .entry(owned.type_name)
            .or_default()
            .extend(owned.variants);
    }

    Ok(grouped
        .into_iter()
        .map(|(type_name, mut variants)| {
            variants.sort_by(|a, b| {
                let a_is_label = a.is_label();
                let b_is_label = b.is_label();
                formatting::compare_with_label_priority(a_is_label, &a.name, b_is_label, &b.name)
            });

            OwnedTypeInfo {
                type_name,
                variants,
            }
        })
        .collect())
}
