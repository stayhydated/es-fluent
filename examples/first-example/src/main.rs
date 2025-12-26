use es_fluent::{EsFluentGenerator, ToFluentString as _};
use example_shared_lib::Languages;
use first_example::first_example::*;
use first_example::i18n;
use strum::IntoEnumIterator as _;

fn main() {
    // EsFluentGenerator::builder()
    //     .build()
    //     .generate()
    //     .expect("Failed to generate FTL files");

    i18n::init();
    Languages::iter().for_each(run);
}

fn run(locale: Languages) {
    i18n::change_locale(locale).unwrap();

    println!("Language: {}", locale.to_fluent_string());

    let hello = HelloUser::new("Alice");
    println!("{}", hello.to_fluent_string());

    for gender in Gender::iter() {
        println!("Gender: {}", gender.to_fluent_string());
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

    println!("{}", shared1.to_fluent_string());
    println!("{}", shared2.to_fluent_string());
    println!("{}", shared3.to_fluent_string());
    println!("{}", shared4.to_fluent_string());

    println!();
}
