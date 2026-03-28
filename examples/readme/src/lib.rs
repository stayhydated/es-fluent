//! Examples from the README

pub mod i18n;

pub mod namespaces;
pub use namespaces::{
    Button, Dialog, FolderStatus, FolderUserProfile, Gender, GenderThis, LoginForm, Status,
    StatusVariants, UserProfile,
};

// #[derive(EsFluent)] - Enums and Structs
use es_fluent::EsFluent;

#[derive(EsFluent)]
pub enum LoginError {
    InvalidPassword, // no params
    UserNotFound {
        username: String,
    }, // exposed as $username in the ftl file
    Something(String, String, String), // exposed as $f0, $f1, $f2 in the ftl file
    SomethingArgNamed(
        #[fluent(arg_name = "input")] String,
        #[fluent(arg_name = "expected")] String,
        #[fluent(arg_name = "details")] String,
    ), // exposed as $input, $expected, $details
}

#[derive(EsFluent)]
pub struct WelcomeMessage<'a> {
    pub name: &'a str, // exposed as $name in the ftl file
    pub count: i32,    // exposed as $count in the ftl file
}

// #[derive(EsFluentChoice)]
use es_fluent::EsFluentChoice;

#[derive(EsFluent, EsFluentChoice)]
#[fluent_choice(serialize_all = "snake_case")]
pub enum GenderChoice {
    Male,
    Female,
    Other,
}

#[derive(EsFluent)]
pub struct Greeting<'a> {
    pub name: &'a str,
    #[fluent(choice)] // Matches $gender -> [male]...
    pub gender: &'a GenderChoice,
}

// #[derive(EsFluentVariants)]
#[derive(EsFluentVariants)]
#[fluent_variants(keys = ["label", "description"])]
pub struct LoginFormVariants {
    pub username: String,
    pub password: String,
}

// #[derive(EsFluentThis)] - origin only
use es_fluent::EsFluentThis;
#[derive(EsFluentThis)]
#[fluent_this(origin)]
pub enum GenderThisOnly {
    Male,
    Female,
    Other,
}

// #[derive(EsFluentThis)] - origin and members combined with EsFluentVariants
use es_fluent::EsFluentVariants;
#[derive(EsFluentThis, EsFluentVariants)]
#[fluent_this(origin, variants)]
#[fluent_variants(keys = ["label", "description"])]
pub struct LoginFormCombined {
    pub username: String,
    pub password: String,
}
