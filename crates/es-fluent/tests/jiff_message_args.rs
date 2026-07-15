#![cfg(all(feature = "derive", feature = "jiff"))]

use es_fluent::registry::{StaticFluentDomain, StaticFluentEntryId};
use es_fluent::{EsFluent, FluentArgs, FluentMessage, FluentValue};
use jiff::{SignedDuration, Span, Timestamp, ToSpan as _, Zoned, civil};
use std::collections::HashMap;

#[derive(EsFluent)]
struct JiffMessage<'a> {
    date: civil::Date,
    borrowed_date: &'a civil::Date,
    date_time: civil::DateTime,
    time: civil::Time,
    timestamp: Timestamp,
    zoned: Zoned,
    maybe_date_present: Option<civil::Date>,
    maybe_date_missing: Option<civil::Date>,
    maybe_borrowed_date_present: Option<&'a civil::Date>,
    maybe_borrowed_date_missing: Option<&'a civil::Date>,
    #[fluent(value = |value: &civil::Date| *value)]
    transformed_date: civil::Date,
    duration: Span,
    signed_duration: SignedDuration,
    borrowed_duration: &'a Span,
    maybe_duration_present: Option<Span>,
    maybe_duration_missing: Option<Span>,
    maybe_borrowed_duration_present: Option<&'a Span>,
    maybe_borrowed_duration_missing: Option<&'a Span>,
    #[fluent(value = |value: &Span| *value)]
    transformed_duration: Span,
}

fn render_args(message: &impl FluentMessage) -> HashMap<String, String> {
    let mut rendered = HashMap::new();
    let intls = intl_memoizer::IntlLangMemoizer::new("en-US".parse().unwrap());
    message.to_fluent_string_with(
        &mut |_domain: StaticFluentDomain,
              _id: StaticFluentEntryId,
              args: Option<&FluentArgs<'_>>| {
            for (name, value) in args.expect("Jiff message arguments").as_raw() {
                let value = match value {
                    FluentValue::String(value) => value.as_ref().to_string(),
                    FluentValue::Custom(value) => value.as_string(&intls).into_owned(),
                    FluentValue::None => "<none>".to_string(),
                    other => panic!("expected a Fluent string, got {other:?}"),
                };
                rendered.insert(name.as_str().to_string(), value);
            }
            "rendered".to_string()
        },
    );
    rendered
}

#[test]
fn derived_messages_accept_owned_borrowed_and_optional_jiff_values() {
    let borrowed_date = civil::date(2026, 7, 15);
    let borrowed_duration = 3.hours().minutes(30);
    let timestamp = "2026-07-14T13:30:15Z".parse::<Timestamp>().unwrap();
    let zoned = "2026-07-14T09:30:15-04:00[America/New_York]"
        .parse::<Zoned>()
        .unwrap();
    let args = render_args(&JiffMessage {
        date: civil::date(2026, 7, 14),
        borrowed_date: &borrowed_date,
        date_time: civil::date(2026, 7, 14).at(9, 30, 15, 0),
        time: civil::time(10, 45, 30, 0),
        timestamp,
        zoned,
        maybe_date_present: Some(civil::date(2026, 7, 16)),
        maybe_date_missing: None,
        maybe_borrowed_date_present: Some(&borrowed_date),
        maybe_borrowed_date_missing: None,
        transformed_date: civil::date(2026, 7, 14),
        duration: 2.hours().minutes(15),
        signed_duration: SignedDuration::from_hours(1) + SignedDuration::from_mins(45),
        borrowed_duration: &borrowed_duration,
        maybe_duration_present: Some(4.hours().minutes(15)),
        maybe_duration_missing: None,
        maybe_borrowed_duration_present: Some(&borrowed_duration),
        maybe_borrowed_duration_missing: None,
        transformed_duration: 5.hours().minutes(45),
    });

    assert_eq!(args["date"], "Jul 14, 2026");
    assert_eq!(args["borrowed_date"], "Jul 15, 2026");
    assert_eq!(args["date_time"], "Jul 14, 2026, 9:30:15\u{202f}AM");
    assert_eq!(args["time"], "10:45:30\u{202f}AM");
    assert_eq!(args["timestamp"], "Jul 14, 2026, 1:30:15\u{202f}PM GMT+0");
    assert_eq!(args["zoned"], "Jul 14, 2026, 9:30:15\u{202f}AM GMT-4");
    assert_eq!(args["maybe_date_present"], "Jul 16, 2026");
    assert_eq!(args["maybe_date_missing"], "<none>");
    assert_eq!(args["maybe_borrowed_date_present"], "Jul 15, 2026");
    assert_eq!(args["maybe_borrowed_date_missing"], "<none>");
    assert_eq!(args["transformed_date"], "Jul 14, 2026");
    assert_eq!(args["duration"], "2h 15m");
    assert_eq!(args["signed_duration"], "1h 45m");
    assert_eq!(args["borrowed_duration"], "3h 30m");
    assert_eq!(args["maybe_duration_present"], "4h 15m");
    assert_eq!(args["maybe_duration_missing"], "<none>");
    assert_eq!(args["maybe_borrowed_duration_present"], "3h 30m");
    assert_eq!(args["maybe_borrowed_duration_missing"], "<none>");
    assert_eq!(args["transformed_duration"], "5h 45m");
}
