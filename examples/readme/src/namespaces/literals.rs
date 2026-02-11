use es_fluent::{EsFluent, EsFluentThis, EsFluentVariants};

#[derive(EsFluent)]
#[fluent(namespace = "ui")]
pub struct Button<'a>(pub &'a str);

#[derive(EsFluentThis)]
#[fluent_this(origin)]
#[fluent(namespace = "forms")]
pub enum GenderThis {
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
