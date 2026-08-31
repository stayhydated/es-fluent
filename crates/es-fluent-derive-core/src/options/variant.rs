use crate::error::{AttrContext, EsFluentCoreResult};
use crate::semantic::{SpannedValue, VariantKey};
use bon::Builder;
use darling::FromMeta;
use getset::Getters;

use super::{PresentFlag, SkipDirective};

/// Closed representation of a message variant's localization behavior.
#[derive(Clone, Debug)]
pub enum MessageVariantDirective {
    Localized {
        key: Option<SpannedValue<VariantKey>>,
    },
    Skipped,
}

impl MessageVariantDirective {
    pub fn key(&self) -> Option<&SpannedValue<VariantKey>> {
        match self {
            Self::Localized { key } => key.as_ref(),
            Self::Skipped => None,
        }
    }

    pub fn variant_key(
        &self,
        _context: AttrContext,
    ) -> EsFluentCoreResult<Option<SpannedValue<VariantKey>>> {
        Ok(self.key().cloned())
    }
}

impl SkipDirective for MessageVariantDirective {
    fn is_skipped(&self) -> bool {
        matches!(self, Self::Skipped)
    }
}

/// Closed representation of generated-variant inclusion behavior.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeneratedVariantDirective {
    Include,
    Skip,
}

impl SkipDirective for GeneratedVariantDirective {
    fn is_skipped(&self) -> bool {
        matches!(self, Self::Skip)
    }
}

#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
pub(crate) struct SkippedVariantAttributeArgs {
    /// Whether to skip this variant.
    #[darling(default)]
    skip: Option<PresentFlag>,
}

impl SkippedVariantAttributeArgs {
    pub(crate) fn directive(&self) -> GeneratedVariantDirective {
        if self.skip.is_some_and(PresentFlag::is_present) {
            GeneratedVariantDirective::Skip
        } else {
            GeneratedVariantDirective::Include
        }
    }
}

#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
pub(crate) struct KeyedVariantAttributeArgs {
    #[darling(flatten)]
    skipped_args: SkippedVariantAttributeArgs,
    /// Overrides the localization key suffix for this variant.
    #[darling(default)]
    key: Option<SpannedValue<VariantKey>>,
}

impl KeyedVariantAttributeArgs {
    pub(super) fn is_skipped(&self) -> bool {
        matches!(
            self.skipped_args.directive(),
            GeneratedVariantDirective::Skip
        )
    }

    pub(super) fn key(&self) -> Option<&SpannedValue<VariantKey>> {
        self.key.as_ref()
    }

    pub(super) fn directive(&self) -> MessageVariantDirective {
        if self.is_skipped() {
            MessageVariantDirective::Skipped
        } else {
            MessageVariantDirective::Localized {
                key: self.key.clone(),
            }
        }
    }
}
