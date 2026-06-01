use es_fluent_derive::{EsFluent, EsFluentChoice, EsFluentLabel, EsFluentVariants};

#[derive(EsFluent)]
pub struct DuplicateFluentField {
    #[fluent(arg = "name", arg = "other")]
    value: String,
}

#[derive(EsFluentVariants)]
#[fluent_variants(keys = ["label"], keys = ["placeholder"])]
pub struct DuplicateVariantsKeys {
    value: String,
}

#[derive(EsFluentLabel)]
#[fluent_label(origin = true, origin = false)]
pub struct DuplicateLabelOrigin;

#[derive(EsFluentChoice)]
#[fluent_choice(rename_all = "snake_case", rename_all = "kebab-case")]
pub enum DuplicateChoice {
    First,
}

fn main() {}
