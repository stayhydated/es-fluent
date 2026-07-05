use es_fluent_derive::EsFluent;

#[derive(EsFluent)]
pub enum LoginError {
    #[fluent(key = "same")]
    MissingUser,
    #[fluent(key = "same")]
    InvalidPassword,
}

fn main() {}
