use darling::{FromDeriveInput, FromMeta};
use getset::Getters;
use heck::{
    ToKebabCase as _, ToLowerCamelCase as _, ToPascalCase as _, ToShoutyKebabCase as _,
    ToShoutySnakeCase as _, ToSnakeCase as _, ToTitleCase as _, ToTrainCase as _,
};
use strum::IntoEnumIterator as _;
use strum::{Display, EnumIter, EnumString, IntoStaticStr};

#[derive(FromDeriveInput, Getters)]
#[darling(supports(enum_unit), attributes(fluent_choice))]
#[getset(get = "pub")]
pub struct ChoiceOpts {
    ident: syn::Ident,
    generics: syn::Generics,
    data: darling::ast::Data<syn::Variant, darling::util::Ignored>,
    #[darling(flatten)]
    attr_args: ChoiceAttributeArgs,
}

#[derive(Default, FromMeta, Getters)]
#[getset(get = "pub")]
pub struct ChoiceAttributeArgs {
    #[darling(default)]
    rename_all: Option<CaseStyle>,
}

#[derive(Clone, Copy, Debug, Display, EnumIter, EnumString, Eq, IntoStaticStr, PartialEq)]
#[strum(const_into_str)]
pub enum CaseStyle {
    #[strum(to_string = "snake_case")]
    SnakeCase,
    #[strum(to_string = "PascalCase")]
    PascalCase,
    #[strum(to_string = "camelCase")]
    CamelCase,
    #[strum(to_string = "kebab-case")]
    KebabCase,
    #[strum(to_string = "SCREAMING_SNAKE_CASE")]
    ScreamingSnakeCase,
    #[strum(to_string = "SCREAMING-KEBAB-CASE")]
    ScreamingKebabCase,
    #[strum(to_string = "Title Case")]
    TitleCase,
    #[strum(to_string = "Train-Case")]
    TrainCase,
    #[strum(to_string = "lowercase")]
    Lowercase,
    #[strum(to_string = "UPPERCASE")]
    Uppercase,
}

impl CaseStyle {
    pub const fn label(self) -> &'static str {
        self.into_str()
    }

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

impl FromMeta for CaseStyle {
    fn from_meta(item: &syn::Meta) -> darling::Result<Self> {
        let (value, _span) = super::string_literal_value(item)?;
        value.parse::<Self>().map_err(|message| {
            let supported = Self::iter()
                .map(CaseStyle::label)
                .collect::<Vec<_>>()
                .join(", ");
            darling::Error::custom(format!(
                "invalid #[fluent_choice(rename_all = ...)] value `{value}`: {message}; supported values are: {supported}"
            ))
            .with_span(item)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::CaseStyle;
    use crate::options::choice::ChoiceOpts;
    use darling::FromDeriveInput as _;
    use syn::{DeriveInput, parse_quote};

    #[test]
    fn case_style_apply_covers_all_variants() {
        const KEBAB_CASE_LABEL: &str = CaseStyle::KebabCase.label();

        assert_eq!(KEBAB_CASE_LABEL, "kebab-case");
        assert_eq!(CaseStyle::SnakeCase.label(), "snake_case");
        assert_eq!(CaseStyle::SnakeCase.apply("HelloWorld"), "hello_world");
        assert_eq!(CaseStyle::PascalCase.apply("hello world"), "HelloWorld");
        assert_eq!(CaseStyle::CamelCase.apply("hello world"), "helloWorld");
        assert_eq!(CaseStyle::KebabCase.apply("HelloWorld"), "hello-world");
        assert_eq!(
            CaseStyle::ScreamingSnakeCase.apply("hello world"),
            "HELLO_WORLD"
        );
        assert_eq!(
            CaseStyle::ScreamingKebabCase.apply("hello world"),
            "HELLO-WORLD"
        );
        assert_eq!(CaseStyle::TitleCase.apply("hello world"), "Hello World");
        assert_eq!(CaseStyle::TrainCase.apply("hello world"), "Hello-World");
        assert_eq!(CaseStyle::Lowercase.apply("Hello_World"), "hello_world");
        assert_eq!(CaseStyle::Uppercase.apply("Hello_World"), "HELLO_WORLD");
    }

    #[test]
    fn choice_options_parse_rename_all_as_case_style() {
        let input: DeriveInput = parse_quote! {
            #[fluent_choice(rename_all = "snake_case")]
            enum Priority {
                VeryHigh,
            }
        };

        let opts = ChoiceOpts::from_derive_input(&input).expect("ChoiceOpts");

        assert!(matches!(
            opts.attr_args().rename_all(),
            Some(CaseStyle::SnakeCase)
        ));
    }

    #[test]
    fn choice_options_reject_invalid_rename_all_during_option_parsing() {
        let input: DeriveInput = parse_quote! {
            #[fluent_choice(rename_all = "not_a_style")]
            enum Priority {
                VeryHigh,
            }
        };

        let err = match ChoiceOpts::from_derive_input(&input) {
            Ok(_) => panic!("invalid style should fail"),
            Err(error) => error,
        };

        assert!(err.to_string().contains("supported values are"));
    }

    #[test]
    fn lowered_choice_model_rejects_unexpected_internal_shapes() {
        let input: DeriveInput = parse_quote! {
            enum Priority {
                High,
            }
        };
        let mut opts = ChoiceOpts::from_derive_input(&input).expect("ChoiceOpts");
        opts.data = darling::ast::Data::Struct(darling::ast::Fields::new(
            darling::ast::Style::Unit,
            Vec::<darling::util::Ignored>::new(),
        ));

        let err = crate::lowered::ChoiceModel::from_options(&opts)
            .expect_err("lowering rejects wrong data shape");
        assert!(err.to_string().contains("must contain enum data"));

        let input: DeriveInput = parse_quote! {
            enum Priority {
                High,
            }
        };
        let mut opts = ChoiceOpts::from_derive_input(&input).expect("ChoiceOpts");
        let darling::ast::Data::Enum(variants) = &mut opts.data else {
            panic!("expected enum data");
        };
        variants[0].fields = syn::Fields::Unnamed(parse_quote!((u8)));

        let err = crate::lowered::ChoiceModel::from_options(&opts)
            .expect_err("lowering rejects non-unit variants");
        assert!(err.to_string().contains("must be unit variants"));
    }
}
