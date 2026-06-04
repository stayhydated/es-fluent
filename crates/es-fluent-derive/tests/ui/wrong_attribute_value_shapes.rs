use es_fluent_derive::{EsFluent, EsFluentChoice, EsFluentLabel, EsFluentVariants};

#[derive(EsFluent)]
pub struct WrongFluentFieldShape {
    #[fluent(arg)]
    value: String,
}

#[derive(EsFluent)]
pub struct WrongBareFlagShapes {
    #[fluent(skip("hidden"))]
    hidden: String,
    #[fluent(selector("kind"))]
    kind: String,
    #[fluent(optional("maybe"))]
    maybe: Option<String>,
}

#[derive(EsFluentLabel)]
#[fluent_label(origin("parent"), variants("children"))]
pub struct WrongLabelFlagShapes;

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
