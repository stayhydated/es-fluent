use es_fluent_derive::{EsFluent, EsFluentChoice, EsFluentVariants};

#[derive(EsFluent)]
pub struct WrongFluentFieldShape {
    #[fluent(arg)]
    value: String,
}

#[derive(EsFluentVariants)]
#[fluent_variants(keys("label"))]
pub struct WrongVariantsKeysShape {
    value: String,
}

#[derive(EsFluentChoice)]
#[fluent_choice(rename_all)]
pub enum WrongChoiceRenameShape {
    First,
}

fn main() {}
