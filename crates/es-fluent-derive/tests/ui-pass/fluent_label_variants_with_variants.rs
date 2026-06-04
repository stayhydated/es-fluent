extern crate es_fluent;

use es_fluent_derive::{EsFluentLabel, EsFluentVariants};

#[derive(EsFluentLabel, EsFluentVariants)]
#[fluent_label(origin, variants)]
pub struct LoginForm<'a, T>
where
    T: 'a,
{
    username: &'a T,
}

fn main() {}
