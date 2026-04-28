use es_fluent::ThisFtl as _;
use example_shared_lib::Languages;
use readme::*;
use strum::IntoEnumIterator as _;

fn main() {
    let i18n = i18n::try_new_with_language(Languages::default()).expect("i18n should initialize");
    Languages::iter().for_each(|language| run(&i18n, language));
}

fn run(i18n: &i18n::I18n, locale: Languages) {
    i18n::change_locale(i18n, locale).unwrap();
    println!("=== Locale: {locale:?} ===");
    println!("=== Deriving Messages ===");
    println!(
        "InvalidPassword: {}",
        i18n.localize_message(&LoginError::InvalidPassword)
    );
    println!(
        "UserNotFound: {}",
        i18n.localize_message(&LoginError::UserNotFound {
            username: "john".to_string()
        })
    );
    println!(
        "Something: {}",
        i18n.localize_message(&LoginError::Something(
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
        ))
    );

    let welcome = WelcomeMessage {
        name: "John",
        count: 5,
    };
    println!("WelcomeMessage: {}", i18n.localize_message(&welcome));
    println!(
        "TransactionError Network: {}",
        i18n.localize_message(&TransactionError::Network(NetworkError::ApiUnavailable))
    );

    println!("\n=== Using Choices ===");
    let greeting = Greeting {
        name: "John",
        gender: &GenderChoice::Male,
    };
    println!("Greeting: {}", i18n.localize_message(&greeting));

    println!("\n=== Generating Variants ===");
    println!(
        "LoginFormVariants Username Label: {}",
        i18n.localize_message(&LoginFormVariantsLabelVariants::Username)
    );
    println!(
        "SettingsTab Notifications: {}",
        i18n.localize_message(&SettingsTabVariants::Notifications)
    );

    println!("\n=== Type-level Keys (This) ===");
    println!("GenderThisOnly: {}", GenderThisOnly::this_ftl(i18n));
    println!(
        "LoginFormCombined Description: {}",
        LoginFormCombinedDescriptionVariants::this_ftl(i18n)
    );
}
