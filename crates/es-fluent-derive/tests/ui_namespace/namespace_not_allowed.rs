use es_fluent_derive::EsFluent;

#[derive(EsFluent)]
#[fluent(namespace = "blocked")]
pub enum NamespaceBlocked {
    A,
}

fn main() {}
