use es_fluent_derive::EsFluent;

#[derive(EsFluent)]
#[fluent(skip_inventory)]
pub enum InternalOnly {
    Hidden,
}

fn main() {}
