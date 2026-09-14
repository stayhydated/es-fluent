use es_fluent_derive::EsFluentVariants;

#[derive(EsFluentVariants)]
#[fluent_variants(skip)]
pub enum LoginError {
    MissingUser,
}

fn main() {}
