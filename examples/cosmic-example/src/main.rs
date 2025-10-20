#![allow(clippy::needless_lifetimes)]

use example_shared_lib::Languages;
mod app;
mod config;
mod i18n;

fn main() -> cosmic::iced::Result {
    i18n::init();
    i18n::change_locale(Languages::default()).unwrap();

    // Settings for configuring the application window and iced runtime.
    let settings = cosmic::app::Settings::default().size_limits(
        cosmic::iced::Limits::NONE
            .min_width(360.0)
            .min_height(180.0),
    );

    // Starts the application's event loop with `()` as the application's flags.
    cosmic::app::run::<app::AppModel>(settings, ())
}
