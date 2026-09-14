use es_fluent_derive::EsFluent;

#[derive(EsFluent)]
pub enum LoginError {
    #[fluent(arg = "username")]
    MissingUser(String),
}

fn main() {}
