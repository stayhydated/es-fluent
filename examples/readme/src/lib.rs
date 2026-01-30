//! Examples from the README

// ============================================================================
// Namespaces Section Examples
// ============================================================================

// EsFluent examples
use es_fluent::EsFluent;

#[derive(EsFluent)]
#[fluent(namespace = "ui")]
pub struct Button<'a>(&'a str);

#[derive(EsFluent)]
#[fluent(namespace = file)]
pub struct Dialog {
    pub title: String,
}

#[derive(EsFluent)]
#[fluent(namespace(file(relative)))]
pub enum Gender {
    Male,
    Female,
    Other(String),
    Helicopter { type_: String },
}

// EsFluentThis examples
use es_fluent::EsFluentThis;

#[derive(EsFluentThis)]
#[fluent_this(origin, namespace = "forms")]
pub enum GenderThis {
    Male,
    Female,
    Other,
}

#[derive(EsFluentThis)]
#[fluent_this(origin, namespace = file)]
pub enum Status {
    Active,
    Inactive,
}

#[derive(EsFluentThis)]
#[fluent_this(origin, namespace(file(relative)))]
pub struct UserProfile;

// EsFluentVariants examples
use es_fluent::EsFluentVariants;

#[derive(EsFluentVariants)]
#[fluent_variants(keys = ["label", "description"], namespace = "forms")]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

#[derive(EsFluentVariants)]
#[fluent_variants(namespace = file)]
pub enum StatusVariants {
    Active,
    Inactive,
}

// ============================================================================
// Derives Section Examples
// ============================================================================

// #[derive(EsFluent)] - Enums and Structs
#[derive(EsFluent)]
pub enum LoginError {
    InvalidPassword,                   // no params
    UserNotFound { username: String }, // exposed as $username in the ftl file
    Something(String, String, String), // exposed as $f1, $f2, $f3 in the ftl file
}

#[derive(EsFluent)]
pub struct UserProfileDerive<'a> {
    pub name: &'a str,   // exposed as $name in the ftl file
    pub gender: &'a str, // exposed as $gender in the ftl file
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
pub struct UserProfileChoice<'a> {
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
#[derive(EsFluentThis)]
#[fluent_this(origin)]
pub enum GenderThisOnly {
    Male,
    Female,
    Other,
}

// #[derive(EsFluentThis)] - origin and members combined with EsFluentVariants
#[derive(EsFluentThis, EsFluentVariants)]
#[fluent_this(origin, members)]
#[fluent_variants(keys = ["label", "description"])]
pub struct LoginFormCombined {
    pub username: String,
    pub password: String,
}
