use es_fluent::ToFluentString as _;
use first_example::first_example::*;
use first_example::i18n;
use strum::IntoEnumIterator as _;

fn main() {
    let mut manager = i18n::init();

    run(&mut manager, "en");

    run(&mut manager, "fr");

    run(&mut manager, "cn");
}

fn run(manager: &mut es_fluent::FluentManager, locale: &str) {
    i18n::change_locale(manager, locale).unwrap();

    println!("Language: {locale}");

    let hello = Hello::User { user_name: "Alice" };
    println!("{}", hello.to_fluent_string());

    for gender in Gender::iter() {
        println!("Gender: {}", gender.to_fluent_string());
    }

    let shared1 = Shared::Photos {
        user_name: "Bob",
        photo_count: &&&&1,
        user_gender: &Gender::Male,
    };
    let shared2 = Shared::Photos {
        user_name: "Carol",
        photo_count: &&&&2,
        user_gender: &Gender::Female,
    };
    let shared3 = Shared::Photos {
        user_name: "Eve",
        photo_count: &&&&5,
        user_gender: &Gender::Other,
    };
    let shared4 = Shared::Photos {
        user_name: "Helicopter",
        photo_count: &&&&5,
        user_gender: &Gender::Helicopter,
    };

    println!("{}", shared1.to_fluent_string());
    println!("{}", shared2.to_fluent_string());
    println!("{}", shared3.to_fluent_string());
    println!("{}", shared4.to_fluent_string());

    println!();
}
