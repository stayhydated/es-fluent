/// Test fixtures for es-fluent-cli tests.
use assert_fs::{TempDir, prelude::*};

pub const CARGO_TOML: &str = include_str!("base/Cargo.toml");
pub const I18N_TOML: &str = include_str!("base/i18n.toml");
pub const HELLO_FTL: &str = include_str!("base/ftl/hello.ftl");
pub const LIB_RS: &str = include_str!("base/lib.rs");

pub fn create_workspace() -> TempDir {
    let temp = TempDir::new().expect("tempdir");
    temp.child("src").create_dir_all().expect("create src");
    temp.child("i18n/en").create_dir_all().expect("create i18n");
    temp.child("Cargo.toml")
        .write_str(CARGO_TOML)
        .expect("write Cargo.toml");
    temp.child("src/lib.rs")
        .write_str(LIB_RS)
        .expect("write lib.rs");
    temp.child("i18n.toml")
        .write_str(I18N_TOML)
        .expect("write i18n.toml");
    temp.child("i18n/en/test-app.ftl")
        .write_str(HELLO_FTL)
        .expect("write ftl");
    temp
}
