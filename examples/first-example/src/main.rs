use example_shared_lib::Languages;
use first_example::first_example::*;
use first_example::i18n;
use strum::IntoEnumIterator as _;

fn main() {
    let i18n = i18n::try_new_with_language(Languages::default()).expect("i18n should initialize");
    Languages::iter().for_each(|language| run(&i18n, language));
}

fn run(i18n: &i18n::I18n, locale: Languages) {
    i18n::change_locale(i18n, locale).unwrap();

    println!("Language: {}", i18n.localize_message(&locale));

    let hello = HelloUser::new("Alice");
    println!("{}", i18n.localize_message(&hello));

    for gender in Gender::iter() {
        println!("Gender: {}", i18n.localize_message(&gender));
    }

    let shared1 = Shared::Photos {
        user_name: "Bob",
        photo_count: &1,
        user_gender: &Gender::Male,
    };
    let shared2 = Shared::Photos {
        user_name: "Carol",
        photo_count: &2,
        user_gender: &Gender::Female,
    };
    let shared3 = Shared::Photos {
        user_name: "Eve",
        photo_count: &5,
        user_gender: &Gender::Other,
    };
    let shared4 = Shared::Photos {
        user_name: "Helicopter",
        photo_count: &67,
        user_gender: &Gender::Helicopter,
    };

    println!("{}", i18n.localize_message(&shared1));
    println!("{}", i18n.localize_message(&shared2));
    println!("{}", i18n.localize_message(&shared3));
    println!("{}", i18n.localize_message(&shared4));

    println!();
}
