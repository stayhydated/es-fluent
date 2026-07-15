#![cfg(all(feature = "chrono", feature = "derive"))]

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use es_fluent::registry::{StaticFluentDomain, StaticFluentEntryId};
use es_fluent::{EsFluent, FluentArgs, FluentMessage, FluentValue};
use std::collections::HashMap;

#[derive(EsFluent)]
struct ChronoMessage<'a> {
    date: NaiveDate,
    borrowed_date: &'a NaiveDate,
    date_time: NaiveDateTime,
    zoned: DateTime<Utc>,
    maybe_date: Option<NaiveDate>,
    maybe_borrowed_date: Option<&'a NaiveDate>,
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
    let args = render_args(&ChronoMessage {
        date,
        borrowed_date: &borrowed_date,
        date_time,
        zoned,
        maybe_date: None,
        maybe_borrowed_date: Some(&borrowed_date),
    });

    assert_eq!(args["date"], "Jul 14, 2026");
    assert_eq!(args["borrowed_date"], "Jul 15, 2026");
    assert_eq!(args["date_time"], "Jul 14, 2026, 9:30:15\u{202f}AM");
    assert_eq!(args["zoned"], "Jul 14, 2026, 9:30:15\u{202f}AM GMT+0");
    assert_eq!(args["maybe_date"], "<none>");
    assert_eq!(args["maybe_borrowed_date"], "Jul 15, 2026");
}
