use crate::{
    FluentValue,
    icu_datetime::{IcuTemporalValue, IcuZonedDateTime, impl_temporal_argument, into_fluent_value},
    traits::{
        FluentArgumentValue, FluentBorrowedArgumentValue, FluentMessageLookup,
        FluentOptionalArgumentValue, IntoFluentArgumentValue,
    },
};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};

fn date(value: NaiveDate) -> IcuTemporalValue {
    let fallback = value.to_string();
    IcuTemporalValue::date(value.into(), fallback)
}

fn date_time(value: NaiveDateTime) -> IcuTemporalValue {
    let fallback = value.to_string();
    IcuTemporalValue::date_time(value.into(), fallback)
}

fn time(value: NaiveTime) -> IcuTemporalValue {
    let fallback = value.to_string();
    IcuTemporalValue::time(value.into(), fallback)
}

fn zoned_date_time<Tz: TimeZone>(value: DateTime<Tz>) -> IcuTemporalValue {
    let fallback = value.to_rfc3339();
    let value = value.fixed_offset();
    IcuTemporalValue::zoned_date_time(IcuZonedDateTime::from(&value), fallback)
}

impl_temporal_argument!(NaiveDate, date);
impl_temporal_argument!(NaiveDateTime, date_time);
impl_temporal_argument!(NaiveTime, time);

impl<'a, Tz> IntoFluentArgumentValue<'a> for FluentArgumentValue<DateTime<Tz>>
where
    Tz: TimeZone,
{
    fn into_fluent_argument_value(
        self,
        _localize: &mut FluentMessageLookup<'_>,
    ) -> FluentValue<'a> {
        into_fluent_value(zoned_date_time(self.into_inner()))
    }
}

impl<'a, 'value, Tz> IntoFluentArgumentValue<'a>
    for FluentBorrowedArgumentValue<'value, DateTime<Tz>>
where
    Tz: TimeZone,
{
    fn into_fluent_argument_value(
        self,
        _localize: &mut FluentMessageLookup<'_>,
    ) -> FluentValue<'a> {
        into_fluent_value(zoned_date_time(self.into_inner().clone()))
    }
}

impl<'a, 'value, 'inner, Tz> IntoFluentArgumentValue<'a>
    for FluentBorrowedArgumentValue<'value, &'inner DateTime<Tz>>
where
    Tz: TimeZone,
{
    fn into_fluent_argument_value(
        self,
        _localize: &mut FluentMessageLookup<'_>,
    ) -> FluentValue<'a> {
        into_fluent_value(zoned_date_time((*self.into_inner()).clone()))
    }
}

impl<'a, Tz> IntoFluentArgumentValue<'a> for FluentOptionalArgumentValue<&DateTime<Tz>>
where
    Tz: TimeZone,
{
    fn into_fluent_argument_value(
        self,
        _localize: &mut FluentMessageLookup<'_>,
    ) -> FluentValue<'a> {
        match self.into_inner() {
            Some(value) => into_fluent_value(zoned_date_time(value.clone())),
            None => FluentValue::None,
        }
    }
}

impl<'a, Tz> IntoFluentArgumentValue<'a> for FluentOptionalArgumentValue<&&DateTime<Tz>>
where
    Tz: TimeZone,
{
    fn into_fluent_argument_value(
        self,
        _localize: &mut FluentMessageLookup<'_>,
    ) -> FluentValue<'a> {
        match self.into_inner() {
            Some(value) => into_fluent_value(zoned_date_time((*value).clone())),
            None => FluentValue::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{FixedOffset, NaiveDate, Utc};
    use fluent_bundle::types::FluentType as _;

    fn localized(value: IcuTemporalValue, language: &str) -> String {
        value
            .as_string(&intl_memoizer::IntlLangMemoizer::new(
                language.parse().unwrap(),
            ))
            .into_owned()
    }

    #[test]
    fn temporal_values_delegate_to_icu4x_locale_formatting() {
        let date_value = NaiveDate::from_ymd_opt(2026, 7, 14).unwrap();
        let value = date(date_value);
        assert_eq!(localized(value.clone(), "en-US"), "Jul 14, 2026");
        assert_eq!(localized(value, "fr-FR"), "14 juil. 2026");

        let date_time_value = date_value.and_hms_opt(9, 30, 15).unwrap();
        let value = date_time(date_time_value);
        assert_eq!(
            localized(value.clone(), "en-US"),
            "Jul 14, 2026, 9:30:15\u{202f}AM"
        );
        assert_eq!(localized(value, "fr-FR"), "14 juil. 2026, 09:30:15");

        let value = time(date_time_value.time());
        assert_eq!(localized(value.clone(), "en-US"), "9:30:15\u{202f}AM");
        assert_eq!(localized(value, "fr-FR"), "09:30:15");
    }

    #[test]
    fn utc_and_fixed_offset_values_use_icu4x_zoned_representation() {
        let date_time = NaiveDate::from_ymd_opt(2026, 7, 14)
            .unwrap()
            .and_hms_opt(13, 30, 15)
            .unwrap();
        let utc = DateTime::<Utc>::from_naive_utc_and_offset(date_time, Utc);
        assert_eq!(
            localized(zoned_date_time(utc), "en-US"),
            "Jul 14, 2026, 1:30:15\u{202f}PM GMT+0"
        );

        let offset = FixedOffset::west_opt(4 * 60 * 60).unwrap();
        let local = offset.from_local_datetime(&date_time).unwrap();
        assert_eq!(
            localized(zoned_date_time(local), "en-US"),
            "Jul 14, 2026, 1:30:15\u{202f}PM GMT-4"
        );
    }
}
