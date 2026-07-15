#![cfg(all(feature = "derive", feature = "icu-datetime"))]

use es_fluent::registry::{StaticFluentDomain, StaticFluentEntryId};
use es_fluent::{EsFluent, FluentArgs, FluentMessage, FluentValue};
use icu_calendar::{Date, Gregorian};
use icu_time::{DateTime, Time};
use std::collections::HashMap;

#[derive(EsFluent)]
struct IcuDateTimeMessage {
    date: Date<Gregorian>,
    date_time: DateTime<Gregorian>,
    time: Time,
}

fn render_args(message: &impl FluentMessage) -> HashMap<String, String> {
    let mut rendered = HashMap::new();
    let intls = intl_memoizer::IntlLangMemoizer::new("en-US".parse().unwrap());
    message.to_fluent_string_with(
        &mut |_domain: StaticFluentDomain,
              _id: StaticFluentEntryId,
              args: Option<&FluentArgs<'_>>| {
            for (name, value) in args.expect("ICU4X message arguments").as_raw() {
                let FluentValue::Custom(value) = value else {
                    panic!("expected an ICU4X Fluent custom value, got {value:?}");
                };
                rendered.insert(
                    name.as_str().to_string(),
                    value.as_string(&intls).into_owned(),
                );
            }
            "rendered".to_string()
        },
    );
    rendered
}

#[test]
fn derived_messages_accept_icu4x_date_time_representations() {
    let date = Date::try_new(2026.into(), 7.into(), 14, Gregorian).unwrap();
    let time = Time::try_new(9, 30, 15, 0).unwrap();
    let args = render_args(&IcuDateTimeMessage {
        date,
        date_time: DateTime { date, time },
        time,
    });

    assert_eq!(args["date"], "Jul 14, 2026");
    assert_eq!(args["date_time"], "Jul 14, 2026, 9:30:15\u{202f}AM");
    assert_eq!(args["time"], "9:30:15\u{202f}AM");
}
