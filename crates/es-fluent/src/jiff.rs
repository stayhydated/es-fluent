use crate::{
    FluentValue,
    icu_datetime::{IcuTemporalValue, IcuZonedDateTime, impl_temporal_argument},
    traits::{
        FluentArgumentValue, FluentBorrowedArgumentValue, FluentMessageLookup,
        FluentOptionalArgumentValue, IntoFluentArgumentValue,
    },
};
use jiff::{SignedDuration, Span, Timestamp, Zoned, civil};

fn date(value: civil::Date) -> IcuTemporalValue {
    let fallback = value.to_string();
    IcuTemporalValue::date(value.into(), fallback)
}

fn date_time(value: civil::DateTime) -> IcuTemporalValue {
    let fallback = value.to_string();
    IcuTemporalValue::date_time(value.into(), fallback)
}

fn time(value: civil::Time) -> IcuTemporalValue {
    let fallback = value.to_string();
    IcuTemporalValue::time(value.into(), fallback)
}

fn timestamp(value: Timestamp) -> IcuTemporalValue {
    let fallback = value.to_string();
    let value = value.to_zoned(jiff::tz::TimeZone::UTC);
    IcuTemporalValue::zoned_date_time(IcuZonedDateTime::from(&value), fallback)
}

fn zoned_date_time(value: Zoned) -> IcuTemporalValue {
    let fallback = value.to_string();
    IcuTemporalValue::zoned_date_time(IcuZonedDateTime::from(&value), fallback)
}

impl_temporal_argument!(civil::Date, date);
impl_temporal_argument!(civil::DateTime, date_time);
impl_temporal_argument!(civil::Time, time);
impl_temporal_argument!(Timestamp, timestamp);
impl_temporal_argument!(Zoned, zoned_date_time);

trait IntoJiffDurationFluentValue {
    fn into_jiff_duration_fluent_value(self) -> FluentValue<'static>;
}

macro_rules! impl_jiff_duration_argument {
    ($ty:ty, $format:expr) => {
        impl IntoJiffDurationFluentValue for $ty {
            fn into_jiff_duration_fluent_value(self) -> FluentValue<'static> {
                ($format)(&self).into()
            }
        }

        impl<'a> IntoFluentArgumentValue<'a> for FluentArgumentValue<$ty> {
            fn into_fluent_argument_value(
                self,
                _localize: &mut FluentMessageLookup<'_>,
            ) -> FluentValue<'a> {
                self.into_inner().into_jiff_duration_fluent_value()
            }
        }

        impl<'a, 'value> IntoFluentArgumentValue<'a> for FluentBorrowedArgumentValue<'value, $ty> {
            fn into_fluent_argument_value(
                self,
                _localize: &mut FluentMessageLookup<'_>,
            ) -> FluentValue<'a> {
                self.into_inner().clone().into_jiff_duration_fluent_value()
            }
        }

        impl<'a, 'value, 'inner> IntoFluentArgumentValue<'a>
            for FluentBorrowedArgumentValue<'value, &'inner $ty>
        {
            fn into_fluent_argument_value(
                self,
                _localize: &mut FluentMessageLookup<'_>,
            ) -> FluentValue<'a> {
                (*self.into_inner())
                    .clone()
                    .into_jiff_duration_fluent_value()
            }
        }

        impl<'a, 'value> IntoFluentArgumentValue<'a> for FluentOptionalArgumentValue<&'value $ty> {
            fn into_fluent_argument_value(
                self,
                _localize: &mut FluentMessageLookup<'_>,
            ) -> FluentValue<'a> {
                match self.into_inner() {
                    Some(value) => value.clone().into_jiff_duration_fluent_value(),
                    None => FluentValue::None,
                }
            }
        }

        impl<'a, 'value, 'inner> IntoFluentArgumentValue<'a>
            for FluentOptionalArgumentValue<&'value &'inner $ty>
        {
            fn into_fluent_argument_value(
                self,
                _localize: &mut FluentMessageLookup<'_>,
            ) -> FluentValue<'a> {
                match self.into_inner() {
                    Some(value) => (*value).clone().into_jiff_duration_fluent_value(),
                    None => FluentValue::None,
                }
            }
        }
    };
}

impl_jiff_duration_argument!(Span, |value: &Span| {
    jiff::fmt::friendly::SpanPrinter::new().span_to_string(value)
});
impl_jiff_duration_argument!(SignedDuration, |value: &SignedDuration| {
    jiff::fmt::friendly::SpanPrinter::new().duration_to_string(value)
});

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_bundle::types::FluentType as _;
    use jiff::ToSpan as _;

    fn localized(value: IcuTemporalValue, language: &str) -> String {
        value
            .as_string(&intl_memoizer::IntlLangMemoizer::new(
                language.parse().unwrap(),
            ))
            .into_owned()
    }

    fn fluent_string(value: impl IntoJiffDurationFluentValue) -> String {
        match value.into_jiff_duration_fluent_value() {
            FluentValue::String(value) => value.into_owned(),
            other => panic!("expected a Fluent string, got {other:?}"),
        }
    }

    #[test]
    fn temporal_values_delegate_to_icu4x_locale_formatting() {
        let value = date(civil::date(2026, 7, 14));
        assert_eq!(localized(value.clone(), "en-US"), "Jul 14, 2026");
        assert_eq!(localized(value, "fr-FR"), "14 juil. 2026");

        let value = date_time(civil::date(2026, 7, 14).at(9, 30, 15, 0));
        assert_eq!(
            localized(value.clone(), "en-US"),
            "Jul 14, 2026, 9:30:15\u{202f}AM"
        );
        assert_eq!(localized(value, "fr-FR"), "14 juil. 2026, 09:30:15");

        let value = time(civil::time(9, 30, 15, 0));
        assert_eq!(localized(value.clone(), "en-US"), "9:30:15\u{202f}AM");
        assert_eq!(localized(value, "fr-FR"), "09:30:15");
    }

    #[test]
    fn instant_and_zoned_values_use_icu4x_zoned_representation() {
        let timestamp = "2026-07-14T13:30:15Z".parse::<Timestamp>().unwrap();
        assert_eq!(
            localized(super::timestamp(timestamp), "en-US"),
            "Jul 14, 2026, 1:30:15\u{202f}PM GMT+0"
        );

        let zoned = "2026-07-14T09:30:15-04:00[America/New_York]"
            .parse::<Zoned>()
            .unwrap();
        assert_eq!(
            localized(zoned_date_time(zoned), "en-US"),
            "Jul 14, 2026, 9:30:15\u{202f}AM GMT-4"
        );
    }

    #[test]
    fn elapsed_values_keep_jiff_friendly_duration_format() {
        assert_eq!(fluent_string(2.hours().minutes(15)), "2h 15m");
        assert_eq!(
            fluent_string(SignedDuration::from_hours(2) + SignedDuration::from_mins(15)),
            "2h 15m"
        );
    }

    #[test]
    fn icu_value_can_be_stored_as_a_fluent_custom_value() {
        assert!(matches!(
            crate::icu_datetime::into_fluent_value(date(civil::date(2026, 7, 14))),
            FluentValue::Custom(_)
        ));
    }

    #[test]
    fn icu_value_formats_through_the_threadsafe_fluent_memoizer() {
        let value = date(civil::date(2026, 7, 14));
        assert_eq!(
            value
                .as_string_threadsafe(&intl_memoizer::concurrent::IntlLangMemoizer::new(
                    "fr-FR".parse().unwrap(),
                ))
                .as_ref(),
            "14 juil. 2026"
        );
    }
}
