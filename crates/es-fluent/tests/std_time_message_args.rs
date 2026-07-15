#![cfg(all(feature = "derive", feature = "icu-datetime"))]

use es_fluent::registry::{StaticFluentDomain, StaticFluentEntryId};
use es_fluent::{EsFluent, FluentArgs, FluentMessage, FluentValue};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(EsFluent)]
struct StdTimeMessage<'a> {
    system_time: SystemTime,
    borrowed_system_time: &'a SystemTime,
    maybe_system_time_present: Option<SystemTime>,
    maybe_system_time_missing: Option<SystemTime>,
    maybe_borrowed_system_time_present: Option<&'a SystemTime>,
    maybe_borrowed_system_time_missing: Option<&'a SystemTime>,
    #[fluent(value = |value: &SystemTime| *value)]
    transformed_system_time: SystemTime,
}

fn render_args(message: &impl FluentMessage) -> HashMap<String, String> {
    let mut rendered = HashMap::new();
    let intls = intl_memoizer::IntlLangMemoizer::new("en-US".parse().unwrap());
    message.to_fluent_string_with(
        &mut |_domain: StaticFluentDomain,
              _id: StaticFluentEntryId,
              args: Option<&FluentArgs<'_>>| {
            for (name, value) in args.expect("std time message arguments").as_raw() {
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
fn derived_messages_accept_owned_borrowed_and_optional_system_times() {
    let system_time = UNIX_EPOCH + Duration::from_secs(1_784_035_815);
    let borrowed_system_time = system_time + Duration::from_secs(3_600);
    let args = render_args(&StdTimeMessage {
        system_time,
        borrowed_system_time: &borrowed_system_time,
        maybe_system_time_present: Some(system_time + Duration::from_secs(86_400)),
        maybe_system_time_missing: None,
        maybe_borrowed_system_time_present: Some(&borrowed_system_time),
        maybe_borrowed_system_time_missing: None,
        transformed_system_time: system_time,
    });

    assert_eq!(args["system_time"], "Jul 14, 2026, 1:30:15\u{202f}PM GMT+0");
    assert_eq!(
        args["borrowed_system_time"],
        "Jul 14, 2026, 2:30:15\u{202f}PM GMT+0"
    );
    assert_eq!(
        args["maybe_system_time_present"],
        "Jul 15, 2026, 1:30:15\u{202f}PM GMT+0"
    );
    assert_eq!(args["maybe_system_time_missing"], "<none>");
    assert_eq!(
        args["maybe_borrowed_system_time_present"],
        "Jul 14, 2026, 2:30:15\u{202f}PM GMT+0"
    );
    assert_eq!(args["maybe_borrowed_system_time_missing"], "<none>");
    assert_eq!(
        args["transformed_system_time"],
        "Jul 14, 2026, 1:30:15\u{202f}PM GMT+0"
    );
}
