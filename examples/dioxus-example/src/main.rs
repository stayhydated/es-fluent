use dioxus::prelude::*;
use dioxus_i18n::use_i18n;
use es_fluent::{EsFluent, FluentManager, ToFluentString};
use std::sync::{Arc, Mutex};
use unic_langid::LanguageIdentifier;

mod i18n;

lazy_static::lazy_static! {
    static ref MANAGER: Arc<Mutex<FluentManager>> = Arc::new(Mutex::new(i18n::init()));
}

fn main() {
    dioxus::launch(App);
}

#[derive(EsFluent)]
pub enum Hello<'a> {
    Title { count: &'a i32 },
    UpHigh,
    DownLow,
}

pub fn App() -> Element {
    let mut count = use_signal(|| 0);
    let mut i18n_context = use_i18n::i18n();

    use_effect(move || {
        let lang = i18n_context.language();
        if let Err(e) = i18n::change_locale(&mut MANAGER.lock().unwrap(), &lang) {
            log::error!("Failed to change locale: {}", e);
        }
    });

    rsx! {
        h1 {
            {
                Hello::Title {
                    count: &count()
                }.to_fluent_string()
            }
        }
        button {
            onclick: move |_| count += 1,
            { Hello::UpHigh.to_fluent_string() }
        }
        button {
            onclick: move |_| count -= 1,
            { Hello::DownLow.to_fluent_string() }
        }

        div {
            margin_top: "10px",
            button {
                onclick: move |_| {
                    let lang = "en".parse::<LanguageIdentifier>().unwrap();
                    i18n_context.set_language(lang.clone());
                    if let Err(e) = i18n::change_locale(&mut MANAGER.lock().unwrap(), &lang) {
                        log::error!("Failed to change locale: {}", e);
                    }
                },
                "English"
            }
            button {
                onclick: move |_| {
                    let lang = "fr".parse::<LanguageIdentifier>().unwrap();
                    i18n_context.set_language(lang.clone());
                    if let Err(e) = i18n::change_locale(&mut MANAGER.lock().unwrap(), &lang) {
                        log::error!("Failed to change locale: {}", e);
                    }
                },
                "Español"
            }
        }
    }
}
