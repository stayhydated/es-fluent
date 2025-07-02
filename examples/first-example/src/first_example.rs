use es_fluent::{EsFluent, EsFluentChoice};
use strum::EnumIter;

#[derive(EsFluent)]
pub enum Hello<'a> {
    User { user_name: &'a str },
}

#[derive(EnumIter, EsFluent, EsFluentChoice)]
#[fluent_choice(serialize_all = "snake_case")]
pub enum Gender {
    Male,
    Female,
    Helicopter,
    Other,
}

#[derive(EsFluent)]
pub enum Shared<'a> {
    Photos {
        user_name: &'a str,
        /// of course! we get this data
        /// from a reference to a reference to a reference to a reference to a u32
        photo_count: &'a &'a &'a &'a u32,
        // this signals the macro to use the choice representation, since we'll
        // match against it in the ftl ressource
        #[fluent(choice)]
        user_gender: &'a Gender,
    },
}
