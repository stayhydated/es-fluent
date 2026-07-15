#![cfg(all(feature = "derive", feature = "icu-datetime"))]

use es_fluent::registry::{StaticFluentDomain, StaticFluentEntryId};
use es_fluent::{EsFluent, FluentArgs, FluentMessage, FluentValue};
use icu_calendar::{Date, Gregorian};
use icu_time::zone::{TimeZoneInfo, models::AtTime};
use icu_time::{DateTime, Time, ZonedDateTime};
use std::collections::HashMap;

#[derive(EsFluent)]
struct IcuDateTimeMessage<'a> {
    date: Date<Gregorian>,
    borrowed_date: &'a Date<Gregorian>,
    date_time: DateTime<Gregorian>,
    time: Time,
    zoned: ZonedDateTime<Gregorian, TimeZoneInfo<AtTime>>,
    maybe_date_present: Option<Date<Gregorian>>,
    maybe_date_missing: Option<Date<Gregorian>>,
    maybe_borrowed_date_present: Option<&'a Date<Gregorian>>,
    maybe_borrowed_date_missing: Option<&'a Date<Gregorian>>,
    #[fluent(value = |value: &Date<Gregorian>| *value)]
    transformed_date: Date<Gregorian>,
}

fn render_args(message: &impl FluentMessage) -> HashMap<String, String> {
    let mut rendered = HashMap::new();
    let intls = intl_memoizer::IntlLangMemoizer::new("en-US".parse().unwrap());
    message.to_fluent_string_with(
        &mut |_domain: StaticFluentDomain,
              _id: StaticFluentEntryId,
              args: Option<&FluentArgs<'_>>| {
            for (name, value) in args.expect("ICU4X message arguments").as_raw() {
                let value = match value {
                    FluentValue::Custom(value) => value.as_string(&intls).into_owned(),
                    FluentValue::None => "<none>".to_string(),
                    other => panic!("expected an ICU4X Fluent custom value, got {other:?}"),
                };
                rendered.insert(name.as_str().to_string(), value);
            }
            "rendered".to_string()
        },
    );
    rendered
}

#[test]
fn derived_messages_accept_icu4x_date_time_representations() {
    let date = Date::try_new(2026.into(), 7.into(), 14, Gregorian).unwrap();
    let borrowed_date = Date::try_new(2026.into(), 7.into(), 15, Gregorian).unwrap();
    let time = Time::try_new(9, 30, 15, 0).unwrap();
    let date_time = DateTime { date, time };
    let args = render_args(&IcuDateTimeMessage {
        date,
        borrowed_date: &borrowed_date,
        date_time,
        time,
        zoned: ZonedDateTime {
            date,
            time,
            zone: TimeZoneInfo::utc().at_date_time(date_time),
        },
        maybe_date_present: Some(borrowed_date),
        maybe_date_missing: None,
        maybe_borrowed_date_present: Some(&borrowed_date),
        maybe_borrowed_date_missing: None,
        transformed_date: date,
    });

    assert_eq!(args["date"], "Jul 14, 2026");
    assert_eq!(args["borrowed_date"], "Jul 15, 2026");
    assert_eq!(args["date_time"], "Jul 14, 2026, 9:30:15\u{202f}AM");
    assert_eq!(args["time"], "9:30:15\u{202f}AM");
    assert_eq!(args["zoned"], "Jul 14, 2026, 9:30:15\u{202f}AM GMT+0");
    assert_eq!(args["maybe_date_present"], "Jul 15, 2026");
    assert_eq!(args["maybe_date_missing"], "<none>");
    assert_eq!(args["maybe_borrowed_date_present"], "Jul 15, 2026");
    assert_eq!(args["maybe_borrowed_date_missing"], "<none>");
    assert_eq!(args["transformed_date"], "Jul 14, 2026");
}
