extern crate es_fluent;

use es_fluent_derive::EsFluentVariants;

#[derive(EsFluentVariants)]
#[fluent_variants(keys = ["label"])]
pub enum LoginState {
    #[fluent_variants(skip)]
    Ready,
}

fn main() {}
