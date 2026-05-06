use es_fluent::{EsFluent, EsFluentLabel, EsFluentVariants};

#[derive(EsFluent)]
#[fluent(namespace = "ui")]
pub struct Button<'a>(pub &'a str);

#[derive(EsFluentLabel)]
#[fluent_label(origin)]
#[fluent(namespace = "forms")]
pub enum GenderLabel {
    Male,
    Female,
    Other,
}

#[derive(EsFluentVariants)]
#[fluent_variants(keys = ["label", "description"])]
#[fluent(namespace = "forms")]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}
