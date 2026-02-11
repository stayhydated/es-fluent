//! Namespace-focused examples are split by declaration file layout so
//! `namespace = file|folder` directives demonstrate realistic paths.

pub mod file_examples;
pub mod literals;
pub mod user;

pub use file_examples::{Dialog, Status, StatusVariants};
pub use literals::{Button, GenderThis, LoginForm};
pub use user::{FolderStatus, FolderUserProfile, Gender, UserProfile};
