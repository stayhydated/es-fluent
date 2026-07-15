#![cfg(all(feature = "derive", feature = "jiff"))]

use es_fluent::registry::{StaticFluentDomain, StaticFluentEntryId};
use es_fluent::{EsFluent, FluentArgs, FluentMessage, FluentValue};
use jiff::{Span, ToSpan as _, civil};
use std::collections::HashMap;

#[derive(EsFluent)]
struct JiffMessage<'a> {
    date: civil::Date,
    borrowed_date: &'a civil::Date,
    duration: Span,
    maybe_duration: Option<Span>,
    maybe_borrowed_duration: Option<&'a Span>,
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
    let args = render_args(&JiffMessage {
        date: civil::date(2026, 7, 14),
        borrowed_date: &borrowed_date,
        duration: 2.hours().minutes(15),
        maybe_duration: None,
        maybe_borrowed_duration: Some(&borrowed_duration),
    });

    assert_eq!(args["date"], "Jul 14, 2026");
    assert_eq!(args["borrowed_date"], "Jul 15, 2026");
    assert_eq!(args["duration"], "2h 15m");
    assert_eq!(args["maybe_duration"], "<none>");
    assert_eq!(args["maybe_borrowed_duration"], "3h 30m");
}
