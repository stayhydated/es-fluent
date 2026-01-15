use darling::{FromDeriveInput, FromMeta};
use getset::Getters;
use heck::{
    ToKebabCase as _, ToLowerCamelCase as _, ToPascalCase as _, ToShoutyKebabCase as _,
    ToShoutySnakeCase as _, ToSnakeCase as _, ToTitleCase as _, ToTrainCase as _,
};
use strum::{Display, EnumIter, EnumString};

#[derive(FromDeriveInput, Getters)]
#[darling(supports(enum_unit), attributes(fluent_choice))]
#[getset(get = "pub")]
pub struct ChoiceOpts {
    pub ident: syn::Ident,
    pub generics: syn::Generics,
    pub data: darling::ast::Data<syn::Variant, darling::util::Ignored>,
    #[darling(flatten)]
    pub attr_args: ChoiceAttributeArgs,
}

#[derive(Default, FromMeta, Getters)]
#[getset(get = "pub")]
pub struct ChoiceAttributeArgs {
    #[darling(default)]
    pub serialize_all: Option<String>,
}

#[derive(Clone, Copy, Debug, Display, EnumIter, EnumString)]
pub enum CaseStyle {
    #[strum(serialize = "snake_case")]
    SnakeCase,
    #[strum(serialize = "PascalCase")]
    PascalCase,
    #[strum(serialize = "camelCase")]
    CamelCase,
    #[strum(serialize = "kebab-case")]
    KebabCase,
    #[strum(serialize = "SCREAMING_SNAKE_CASE")]
    ScreamingSnakeCase,
    #[strum(serialize = "SCREAMING-KEBAB-CASE")]
    ScreamingKebabCase,
    #[strum(serialize = "Title Case")]
    TitleCase,
    #[strum(serialize = "Train-Case")]
    TrainCase,
    #[strum(serialize = "lowercase")]
    Lowercase,
    #[strum(serialize = "UPPERCASE")]
    Uppercase,
}

impl CaseStyle {
    pub fn apply(&self, s: &str) -> String {
        match self {
            CaseStyle::SnakeCase => s.to_snake_case(),
            CaseStyle::PascalCase => s.to_pascal_case(),
            CaseStyle::CamelCase => s.to_lower_camel_case(),
            CaseStyle::KebabCase => s.to_kebab_case(),
            CaseStyle::ScreamingSnakeCase => s.to_shouty_snake_case(),
            CaseStyle::ScreamingKebabCase => s.to_shouty_kebab_case(),
            CaseStyle::TitleCase => s.to_title_case(),
            CaseStyle::TrainCase => s.to_train_case(),
            CaseStyle::Lowercase => s.to_lowercase(),
            CaseStyle::Uppercase => s.to_uppercase(),
        }
    }
}
