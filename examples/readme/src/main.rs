use es_fluent::{ThisFtl as _, ToFluentString as _};
use example_shared_lib::Languages;
use readme::*;
use strum::IntoEnumIterator as _;

fn main() {
    i18n::init_with_language(Languages::default());
    Languages::iter().for_each(run);
}

fn run(locale: Languages) {
    i18n::change_locale(locale).unwrap();
    // Escaping the typical macro generation checks, we can test and print
    // what string paths each type evaluates to:
    println!("=== Deriving Messages ===");
    println!(
        "InvalidPassword: {}",
        LoginError::InvalidPassword.to_fluent_string()
    );
    println!(
        "UserNotFound: {}",
        LoginError::UserNotFound {
            username: "john".to_string()
        }
        .to_fluent_string()
    );
    println!(
        "Something: {}",
        LoginError::Something("a".to_string(), "b".to_string(), "c".to_string()).to_fluent_string()
    );

    let welcome = WelcomeMessage {
        name: "John",
        count: 5,
    };
    println!("WelcomeMessage: {}", welcome.to_fluent_string());

    println!("\n=== Using Choices ===");
    let greeting = Greeting {
        name: "John",
        gender: &GenderChoice::Male,
    };
    println!("Greeting: {}", greeting.to_fluent_string());

    println!("\n=== Generating Variants ===");
    println!(
        "LoginFormVariants Username Label: {}",
        LoginFormVariantsLabelVariants::Username.to_fluent_string()
    );
    println!(
        "SettingsTab Notifications: {}",
        SettingsTabVariants::Notifications.to_fluent_string()
    );

    println!("\n=== Type-level Keys (This) ===");
    println!("GenderThisOnly: {}", GenderThisOnly::this_ftl());
    println!(
        "LoginFormCombined Description: {}",
        LoginFormCombinedDescriptionVariants::this_ftl()
    );
}
