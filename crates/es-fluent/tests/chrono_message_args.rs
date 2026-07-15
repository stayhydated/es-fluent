#![cfg(all(feature = "chrono", feature = "derive"))]

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use es_fluent::registry::{StaticFluentDomain, StaticFluentEntryId};
use es_fluent::{EsFluent, FluentArgs, FluentMessage, FluentValue};
use std::collections::HashMap;

#[derive(EsFluent)]
struct ChronoMessage<'a> {
    date: NaiveDate,
    borrowed_date: &'a NaiveDate,
    date_time: NaiveDateTime,
    time: NaiveTime,
    zoned: DateTime<Utc>,
    borrowed_zoned: &'a DateTime<Utc>,
    maybe_date_present: Option<NaiveDate>,
    maybe_date_missing: Option<NaiveDate>,
    maybe_borrowed_date_present: Option<&'a NaiveDate>,
    maybe_borrowed_date_missing: Option<&'a NaiveDate>,
    maybe_zoned_present: Option<DateTime<Utc>>,
    maybe_zoned_missing: Option<DateTime<Utc>>,
    maybe_borrowed_zoned_present: Option<&'a DateTime<Utc>>,
    maybe_borrowed_zoned_missing: Option<&'a DateTime<Utc>>,
    #[fluent(value = |value: &NaiveDate| *value)]
    transformed_date: NaiveDate,
    #[fluent(value = |value: &DateTime<Utc>| value.to_owned())]
    transformed_zoned: DateTime<Utc>,
}

fn render_args(message: &impl FluentMessage) -> HashMap<String, String> {
    let mut rendered = HashMap::new();
    let intls = intl_memoizer::IntlLangMemoizer::new("en-US".parse().unwrap());
    message.to_fluent_string_with(
        &mut |_domain: StaticFluentDomain,
              _id: StaticFluentEntryId,
              args: Option<&FluentArgs<'_>>| {
            for (name, value) in args.expect("Chrono message arguments").as_raw() {
                let value = match value {
                    FluentValue::String(value) => value.as_ref().to_string(),
                    FluentValue::Custom(value) => value.as_string(&intls).into_owned(),
                    FluentValue::None => "<none>".to_string(),
                    other => panic!("expected a Fluent temporal value, got {other:?}"),
                };
                rendered.insert(name.as_str().to_string(), value);
            }
            "rendered".to_string()
        },
    );
    rendered
}

#[test]
fn derived_messages_accept_owned_borrowed_and_optional_chrono_values() {
    let date = NaiveDate::from_ymd_opt(2026, 7, 14).unwrap();
    let borrowed_date = NaiveDate::from_ymd_opt(2026, 7, 15).unwrap();
    let date_time = date.and_hms_opt(9, 30, 15).unwrap();
    let zoned = DateTime::<Utc>::from_naive_utc_and_offset(date_time, Utc);
    let borrowed_zoned = DateTime::<Utc>::from_naive_utc_and_offset(
        borrowed_date.and_hms_opt(10, 45, 30).unwrap(),
        Utc,
    );
    let args = render_args(&ChronoMessage {
        date,
        borrowed_date: &borrowed_date,
        date_time,
        time: NaiveTime::from_hms_opt(10, 45, 30).unwrap(),
        zoned,
        borrowed_zoned: &borrowed_zoned,
        maybe_date_present: Some(NaiveDate::from_ymd_opt(2026, 7, 16).unwrap()),
        maybe_date_missing: None,
        maybe_borrowed_date_present: Some(&borrowed_date),
        maybe_borrowed_date_missing: None,
        maybe_zoned_present: Some(zoned),
        maybe_zoned_missing: None,
        maybe_borrowed_zoned_present: Some(&borrowed_zoned),
        maybe_borrowed_zoned_missing: None,
        transformed_date: date,
        transformed_zoned: zoned,
    });

    assert_eq!(args["date"], "Jul 14, 2026");
    assert_eq!(args["borrowed_date"], "Jul 15, 2026");
    assert_eq!(args["date_time"], "Jul 14, 2026, 9:30:15\u{202f}AM");
    assert_eq!(args["time"], "10:45:30\u{202f}AM");
    assert_eq!(args["zoned"], "Jul 14, 2026, 9:30:15\u{202f}AM GMT+0");
    assert_eq!(
        args["borrowed_zoned"],
        "Jul 15, 2026, 10:45:30\u{202f}AM GMT+0"
    );
    assert_eq!(args["maybe_date_present"], "Jul 16, 2026");
    assert_eq!(args["maybe_date_missing"], "<none>");
    assert_eq!(args["maybe_borrowed_date_present"], "Jul 15, 2026");
    assert_eq!(args["maybe_borrowed_date_missing"], "<none>");
    assert_eq!(
        args["maybe_zoned_present"],
        "Jul 14, 2026, 9:30:15\u{202f}AM GMT+0"
    );
    assert_eq!(args["maybe_zoned_missing"], "<none>");
    assert_eq!(
        args["maybe_borrowed_zoned_present"],
        "Jul 15, 2026, 10:45:30\u{202f}AM GMT+0"
    );
    assert_eq!(args["maybe_borrowed_zoned_missing"], "<none>");
    assert_eq!(args["transformed_date"], "Jul 14, 2026");
    assert_eq!(
        args["transformed_zoned"],
        "Jul 14, 2026, 9:30:15\u{202f}AM GMT+0"
    );
}
