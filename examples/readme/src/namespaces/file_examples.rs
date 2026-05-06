use es_fluent::{EsFluent, EsFluentLabel, EsFluentVariants};

#[derive(EsFluent)]
#[fluent(namespace = file)]
pub struct Dialog {
    pub title: String,
}

#[derive(EsFluentLabel)]
#[fluent_label(origin)]
#[fluent(namespace = file)]
pub enum Status {
    Active,
    Inactive,
}

#[derive(EsFluentVariants)]
#[fluent(namespace = file)]
pub enum StatusVariants {
    Active,
    Inactive,
}
