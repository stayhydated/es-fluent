use crate::FluentValue;
use ::icu_calendar::Gregorian;
use ::icu_datetime::{DateTimeFormatter, DateTimeFormatterPreferences, fieldsets};
use ::icu_time::zone::{TimeZoneInfo, models::AtTime};
use fluent_bundle::types::FluentType;
use intl_memoizer::Memoizable;
use std::{
    borrow::Cow,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use writeable::Writeable as _;

pub(crate) type IcuDate = ::icu_datetime::input::Date<Gregorian>;
pub(crate) type IcuDateTime = ::icu_datetime::input::DateTime<Gregorian>;
pub(crate) type IcuTime = ::icu_datetime::input::Time;
pub(crate) type IcuZonedDateTime =
    ::icu_datetime::input::ZonedDateTime<Gregorian, TimeZoneInfo<AtTime>>;

#[derive(Clone, Debug)]
enum IcuTemporalRepresentation {
    Date(IcuDate),
    DateTime(IcuDateTime),
    Time(IcuTime),
    ZonedDateTime(IcuZonedDateTime),
}

#[derive(Clone, Debug)]
pub(crate) struct IcuTemporalValue {
    representation: IcuTemporalRepresentation,
    fallback: String,
}

impl IcuTemporalValue {
    pub(crate) fn date(value: IcuDate, fallback: String) -> Self {
        Self {
            representation: IcuTemporalRepresentation::Date(value),
            fallback,
        }
    }

    pub(crate) fn date_time(value: IcuDateTime, fallback: String) -> Self {
        Self {
            representation: IcuTemporalRepresentation::DateTime(value),
            fallback,
        }
    }

    pub(crate) fn time(value: IcuTime, fallback: String) -> Self {
        Self {
            representation: IcuTemporalRepresentation::Time(value),
            fallback,
        }
    }

    pub(crate) fn zoned_date_time(value: IcuZonedDateTime, fallback: String) -> Self {
        Self {
            representation: IcuTemporalRepresentation::ZonedDateTime(value),
            fallback,
        }
    }

    fn kind(&self) -> u8 {
        match self.representation {
            IcuTemporalRepresentation::Date(_) => 0,
            IcuTemporalRepresentation::DateTime(_) => 1,
            IcuTemporalRepresentation::Time(_) => 2,
            IcuTemporalRepresentation::ZonedDateTime(_) => 3,
        }
    }

    fn format(&self, intls: &intl_memoizer::IntlLangMemoizer) -> String {
        let formatted = match &self.representation {
            IcuTemporalRepresentation::Date(value) => intls
                .with_try_get::<IcuDateFormatter, _, _>((), |formatter| {
                    formatter.0.format(value).write_to_string().into_owned()
                }),
            IcuTemporalRepresentation::DateTime(value) => intls
                .with_try_get::<IcuDateTimeFormatter, _, _>((), |formatter| {
                    formatter.0.format(value).write_to_string().into_owned()
                }),
            IcuTemporalRepresentation::Time(value) => intls
                .with_try_get::<IcuTimeFormatter, _, _>((), |formatter| {
                    formatter.0.format(value).write_to_string().into_owned()
                }),
            IcuTemporalRepresentation::ZonedDateTime(value) => intls
                .with_try_get::<IcuZonedDateTimeFormatter, _, _>((), |formatter| {
                    formatter.0.format(value).write_to_string().into_owned()
                }),
        };
        formatted.unwrap_or_else(|_| self.fallback.clone())
    }

    fn format_threadsafe(&self, intls: &intl_memoizer::concurrent::IntlLangMemoizer) -> String {
        let formatted = match &self.representation {
            IcuTemporalRepresentation::Date(value) => intls
                .with_try_get::<IcuDateFormatter, _, _>((), |formatter| {
                    formatter.0.format(value).write_to_string().into_owned()
                }),
            IcuTemporalRepresentation::DateTime(value) => intls
                .with_try_get::<IcuDateTimeFormatter, _, _>((), |formatter| {
                    formatter.0.format(value).write_to_string().into_owned()
                }),
            IcuTemporalRepresentation::Time(value) => intls
                .with_try_get::<IcuTimeFormatter, _, _>((), |formatter| {
                    formatter.0.format(value).write_to_string().into_owned()
                }),
            IcuTemporalRepresentation::ZonedDateTime(value) => intls
                .with_try_get::<IcuZonedDateTimeFormatter, _, _>((), |formatter| {
                    formatter.0.format(value).write_to_string().into_owned()
                }),
        };
        formatted.unwrap_or_else(|_| self.fallback.clone())
    }
}

impl PartialEq for IcuTemporalValue {
    fn eq(&self, other: &Self) -> bool {
        self.kind() == other.kind() && self.fallback == other.fallback
    }
}

impl FluentType for IcuTemporalValue {
    fn duplicate(&self) -> Box<dyn FluentType + Send> {
        Box::new(self.clone())
    }

    fn as_string(&self, intls: &intl_memoizer::IntlLangMemoizer) -> Cow<'static, str> {
        Cow::Owned(self.format(intls))
    }

    fn as_string_threadsafe(
        &self,
        intls: &intl_memoizer::concurrent::IntlLangMemoizer,
    ) -> Cow<'static, str> {
        Cow::Owned(self.format_threadsafe(intls))
    }
}

pub(crate) fn into_fluent_value(value: IcuTemporalValue) -> FluentValue<'static> {
    FluentValue::Custom(Box::new(value))
}

struct IcuDateFormatter(DateTimeFormatter<fieldsets::YMD>);
struct IcuDateTimeFormatter(DateTimeFormatter<fieldsets::YMDT>);
struct IcuTimeFormatter(DateTimeFormatter<fieldsets::T>);
struct IcuZonedDateTimeFormatter(
    DateTimeFormatter<fieldsets::Combo<fieldsets::YMDT, fieldsets::zone::LocalizedOffsetShort>>,
);

fn preferences(
    language: unic_langid::LanguageIdentifier,
) -> Result<DateTimeFormatterPreferences, String> {
    language
        .to_string()
        .parse::<icu_locale::Locale>()
        .map(Into::into)
        .map_err(|error| error.to_string())
}

macro_rules! impl_memoizable_formatter {
    ($formatter:ty, $field_set:expr) => {
        impl Memoizable for $formatter {
            type Args = ();
            type Error = String;

            fn construct(
                language: unic_langid::LanguageIdentifier,
                (): Self::Args,
            ) -> Result<Self, Self::Error> {
                DateTimeFormatter::try_new(preferences(language)?, $field_set)
                    .map(Self)
                    .map_err(|error| error.to_string())
            }
        }
    };
}

impl_memoizable_formatter!(IcuDateFormatter, fieldsets::YMD::medium());
impl_memoizable_formatter!(IcuDateTimeFormatter, fieldsets::YMDT::medium());
impl_memoizable_formatter!(IcuTimeFormatter, fieldsets::T::medium());
impl_memoizable_formatter!(
    IcuZonedDateTimeFormatter,
    fieldsets::YMDT::medium().with_zone(fieldsets::zone::LocalizedOffsetShort)
);

macro_rules! impl_temporal_argument {
    ($ty:ty, $convert:path) => {
        impl<'a> $crate::traits::IntoFluentArgumentValue<'a>
            for $crate::traits::FluentArgumentValue<$ty>
        {
            fn into_fluent_argument_value(
                self,
                _localize: &mut $crate::traits::FluentMessageLookup<'_>,
            ) -> $crate::FluentValue<'a> {
                $crate::icu_datetime::into_fluent_value($convert(self.into_inner()))
            }
        }

        impl<'a, 'value> $crate::traits::IntoFluentArgumentValue<'a>
            for $crate::traits::FluentBorrowedArgumentValue<'value, $ty>
        {
            fn into_fluent_argument_value(
                self,
                _localize: &mut $crate::traits::FluentMessageLookup<'_>,
            ) -> $crate::FluentValue<'a> {
                $crate::icu_datetime::into_fluent_value($convert(self.into_inner().clone()))
            }
        }

        impl<'a, 'value, 'inner> $crate::traits::IntoFluentArgumentValue<'a>
            for $crate::traits::FluentBorrowedArgumentValue<'value, &'inner $ty>
        {
            fn into_fluent_argument_value(
                self,
                _localize: &mut $crate::traits::FluentMessageLookup<'_>,
            ) -> $crate::FluentValue<'a> {
                $crate::icu_datetime::into_fluent_value($convert((*self.into_inner()).clone()))
            }
        }

        impl<'a, 'value> $crate::traits::IntoFluentArgumentValue<'a>
            for $crate::traits::FluentOptionalArgumentValue<&'value $ty>
        {
            fn into_fluent_argument_value(
                self,
                _localize: &mut $crate::traits::FluentMessageLookup<'_>,
            ) -> $crate::FluentValue<'a> {
                match self.into_inner() {
                    Some(value) => $crate::icu_datetime::into_fluent_value($convert(value.clone())),
                    None => $crate::FluentValue::None,
                }
            }
        }

        impl<'a, 'value, 'inner> $crate::traits::IntoFluentArgumentValue<'a>
            for $crate::traits::FluentOptionalArgumentValue<&'value &'inner $ty>
        {
            fn into_fluent_argument_value(
                self,
                _localize: &mut $crate::traits::FluentMessageLookup<'_>,
            ) -> $crate::FluentValue<'a> {
                match self.into_inner() {
                    Some(value) => {
                        $crate::icu_datetime::into_fluent_value($convert((*value).clone()))
                    },
                    None => $crate::FluentValue::None,
                }
            }
        }
    };
}

#[cfg(any(feature = "chrono", feature = "jiff"))]
pub(crate) use impl_temporal_argument;

fn icu_date(value: IcuDate) -> IcuTemporalValue {
    let fallback = format!("{value:?}");
    IcuTemporalValue::date(value, fallback)
}

fn icu_date_time(value: IcuDateTime) -> IcuTemporalValue {
    let fallback = format!("{value:?}");
    IcuTemporalValue::date_time(value, fallback)
}

fn icu_time(value: IcuTime) -> IcuTemporalValue {
    let fallback = format!("{value:?}");
    IcuTemporalValue::time(value, fallback)
}

fn icu_zoned_date_time(value: IcuZonedDateTime) -> IcuTemporalValue {
    let fallback = format!("{value:?}");
    IcuTemporalValue::zoned_date_time(value, fallback)
}

fn duration_milliseconds(duration: Duration) -> i128 {
    i128::from(duration.as_secs()) * 1_000 + i128::from(duration.subsec_millis())
}

fn system_time_epoch_milliseconds(value: SystemTime) -> i64 {
    let milliseconds = match value.duration_since(UNIX_EPOCH) {
        Ok(duration) => duration_milliseconds(duration),
        Err(error) => {
            let duration = error.duration();
            let submillisecond = duration.subsec_nanos() % 1_000_000;
            -duration_milliseconds(duration) - i128::from(submillisecond != 0)
        },
    };

    i64::try_from(milliseconds).unwrap_or(if milliseconds.is_negative() {
        i64::MIN
    } else {
        i64::MAX
    })
}

fn system_time(value: SystemTime) -> IcuTemporalValue {
    let fallback = format!("{value:?}");
    let utc: ::icu_time::ZonedDateTime<::icu_calendar::Iso, ::icu_time::zone::UtcOffset> =
        ::icu_time::ZonedDateTime::from_epoch_milliseconds_and_utc_offset(
            system_time_epoch_milliseconds(value),
            ::icu_time::zone::UtcOffset::zero(),
        );
    let date = utc.date.to_calendar(Gregorian);
    let time = utc.time;
    let date_time = IcuDateTime { date, time };
    let zone = TimeZoneInfo::utc().at_date_time(date_time);

    IcuTemporalValue::zoned_date_time(IcuZonedDateTime { date, time, zone }, fallback)
}

impl_temporal_argument!(IcuDate, icu_date);
impl_temporal_argument!(IcuDateTime, icu_date_time);
impl_temporal_argument!(IcuTime, icu_time);
impl_temporal_argument!(IcuZonedDateTime, icu_zoned_date_time);
impl_temporal_argument!(SystemTime, system_time);

#[cfg(test)]
mod tests {
    use super::*;
    use ::icu_time::zone::TimeZoneInfo;

    fn localized(value: IcuTemporalValue, language: &str) -> String {
        value
            .as_string(&intl_memoizer::IntlLangMemoizer::new(
                language.parse().unwrap(),
            ))
            .into_owned()
    }

    fn temporal_values() -> [IcuTemporalValue; 4] {
        let date = IcuDate::try_new(2026.into(), 7.into(), 14, Gregorian).unwrap();
        let time = IcuTime::try_new(9, 30, 15, 0).unwrap();
        let date_time = IcuDateTime { date, time };
        let zoned_date_time = IcuZonedDateTime {
            date,
            time,
            zone: TimeZoneInfo::utc().at_date_time(date_time),
        };

        [
            icu_date(date),
            icu_date_time(date_time),
            icu_time(time),
            icu_zoned_date_time(zoned_date_time),
        ]
    }

    #[test]
    fn fluent_custom_values_clone_with_equal_temporal_representations() {
        for temporal_value in temporal_values() {
            let fluent_value = into_fluent_value(temporal_value);
            assert_eq!(fluent_value, fluent_value.clone());
        }
    }

    #[test]
    fn all_temporal_representations_format_with_the_threadsafe_memoizer() {
        let intls = intl_memoizer::concurrent::IntlLangMemoizer::new("en-US".parse().unwrap());
        let formatted: Vec<_> = temporal_values()
            .into_iter()
            .map(|value| value.as_string_threadsafe(&intls).into_owned())
            .collect();

        assert_eq!(
            formatted,
            [
                "Jul 14, 2026",
                "Jul 14, 2026, 9:30:15\u{202f}AM",
                "9:30:15\u{202f}AM",
                "Jul 14, 2026, 9:30:15\u{202f}AM GMT+0",
            ]
        );
    }

    #[test]
    fn system_time_epoch_milliseconds_round_toward_the_past() {
        assert_eq!(system_time_epoch_milliseconds(UNIX_EPOCH), 0);
        assert_eq!(
            system_time_epoch_milliseconds(UNIX_EPOCH + Duration::from_nanos(1_500_001)),
            1
        );
        assert_eq!(
            system_time_epoch_milliseconds(UNIX_EPOCH - Duration::from_nanos(1)),
            -1
        );
        assert_eq!(
            system_time_epoch_milliseconds(UNIX_EPOCH - Duration::from_nanos(1_000_001)),
            -2
        );
    }

    #[test]
    fn system_time_formats_as_a_utc_offset_on_both_sides_of_the_unix_epoch() {
        assert_eq!(
            localized(
                system_time(UNIX_EPOCH + Duration::from_secs(1_784_035_815)),
                "en-US"
            ),
            "Jul 14, 2026, 1:30:15\u{202f}PM GMT+0"
        );
        assert_eq!(
            localized(
                system_time(UNIX_EPOCH - Duration::from_millis(250)),
                "en-US"
            ),
            "Dec 31, 1969, 11:59:59\u{202f}PM GMT+0"
        );
    }
}
