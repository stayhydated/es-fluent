use crate::FluentValue;
use ::icu_calendar::Gregorian;
use ::icu_datetime::{DateTimeFormatter, DateTimeFormatterPreferences, fieldsets};
use ::icu_experimental::duration::{
    Duration as IcuDuration, DurationFormatter, ValidatedDurationFormatterOptions,
    options::{DurationFormatterOptions, FieldDisplay},
};
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
    Duration(IcuDuration),
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

    pub(crate) fn duration(value: IcuDuration, fallback: String) -> Self {
        Self {
            representation: IcuTemporalRepresentation::Duration(value),
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
            IcuTemporalRepresentation::Duration(_) => 2,
            IcuTemporalRepresentation::Time(_) => 3,
            IcuTemporalRepresentation::ZonedDateTime(_) => 4,
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
            IcuTemporalRepresentation::Duration(value) if value == &IcuDuration::default() => intls
                .with_try_get::<IcuZeroDurationFormatter, _, _>((), |formatter| {
                    formatter.0.format(value).write_to_string().into_owned()
                }),
            IcuTemporalRepresentation::Duration(value) => intls
                .with_try_get::<IcuDurationFormatter, _, _>((), |formatter| {
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
            IcuTemporalRepresentation::Duration(value) if value == &IcuDuration::default() => intls
                .with_try_get::<IcuZeroDurationFormatter, _, _>((), |formatter| {
                    formatter.0.format(value).write_to_string().into_owned()
                }),
            IcuTemporalRepresentation::Duration(value) => intls
                .with_try_get::<IcuDurationFormatter, _, _>((), |formatter| {
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
struct IcuDurationFormatter(DurationFormatter);
struct IcuZeroDurationFormatter(DurationFormatter);
struct IcuTimeFormatter(DateTimeFormatter<fieldsets::T>);
struct IcuZonedDateTimeFormatter(
    DateTimeFormatter<fieldsets::Combo<fieldsets::YMDT, fieldsets::zone::LocalizedOffsetShort>>,
);

fn locale(language: unic_langid::LanguageIdentifier) -> Result<icu_locale::Locale, String> {
    language
        .to_string()
        .parse::<icu_locale::Locale>()
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
                let preferences: DateTimeFormatterPreferences = locale(language)?.into();
                DateTimeFormatter::try_new(preferences, $field_set)
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

fn duration_formatter(
    language: unic_langid::LanguageIdentifier,
    show_zero_seconds: bool,
) -> Result<DurationFormatter, String> {
    let mut options = DurationFormatterOptions::default();
    if show_zero_seconds {
        options.second_visibility = Some(FieldDisplay::Always);
    }

    let options =
        ValidatedDurationFormatterOptions::validate(options).map_err(|error| error.to_string())?;
    DurationFormatter::try_new(locale(language)?.into(), options).map_err(|error| error.to_string())
}

macro_rules! impl_memoizable_duration_formatter {
    ($formatter:ty, $show_zero_seconds:literal) => {
        impl Memoizable for $formatter {
            type Args = ();
            type Error = String;

            fn construct(
                language: unic_langid::LanguageIdentifier,
                (): Self::Args,
            ) -> Result<Self, Self::Error> {
                duration_formatter(language, $show_zero_seconds).map(Self)
            }
        }
    };
}

impl_memoizable_duration_formatter!(IcuDurationFormatter, false);
impl_memoizable_duration_formatter!(IcuZeroDurationFormatter, true);

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

fn std_duration(value: Duration) -> IcuTemporalValue {
    let fallback = format!("{value:?}");
    let seconds = value.as_secs();
    let subsecond_nanoseconds = u64::from(value.subsec_nanos());
    let duration = IcuDuration {
        hours: seconds / 3_600,
        minutes: seconds / 60 % 60,
        seconds: seconds % 60,
        milliseconds: subsecond_nanoseconds / 1_000_000,
        microseconds: subsecond_nanoseconds / 1_000 % 1_000,
        nanoseconds: subsecond_nanoseconds % 1_000,
        ..Default::default()
    };
    IcuTemporalValue::duration(duration, fallback)
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
impl_temporal_argument!(Duration, std_duration);
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

    fn temporal_values() -> [IcuTemporalValue; 5] {
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
            std_duration(Duration::from_secs(3_723)),
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
                "1 hr, 2 min, 3 sec",
                "9:30:15\u{202f}AM",
                "Jul 14, 2026, 9:30:15\u{202f}AM GMT+0",
            ]
        );
    }

    #[test]
    fn std_duration_uses_locale_aware_short_formatting() {
        let value = std_duration(Duration::new(3_723, 456_789_123));

        assert_eq!(
            localized(value.clone(), "en-US"),
            "1 hr, 2 min, 3 sec, 456 ms, 789 μs, 123 ns"
        );
        assert_eq!(
            localized(value, "fr-FR"),
            "1\u{202f}h, 2\u{a0}min, 3\u{202f}s, 456\u{202f}ms, 789\u{202f}μs et 123\u{202f}ns"
        );
    }

    #[test]
    fn zero_std_duration_keeps_a_localized_seconds_unit() {
        let value = std_duration(Duration::ZERO);
        assert_eq!(localized(value.clone(), "en-US"), "0 sec");
        assert_eq!(localized(value.clone(), "fr-FR"), "0\u{202f}s");
        let intls = intl_memoizer::concurrent::IntlLangMemoizer::new("en-US".parse().unwrap());
        assert_eq!(value.as_string_threadsafe(&intls), "0 sec");
    }

    #[test]
    fn std_duration_balances_at_hours_and_preserves_subsecond_units() {
        let value = std_duration(Duration::new(90_061, 123_456_789));
        let IcuTemporalRepresentation::Duration(duration) = value.representation else {
            panic!("expected an ICU4X duration representation");
        };

        assert_eq!(duration.hours, 25);
        assert_eq!(duration.minutes, 1);
        assert_eq!(duration.seconds, 1);
        assert_eq!(duration.milliseconds, 123);
        assert_eq!(duration.microseconds, 456);
        assert_eq!(duration.nanoseconds, 789);
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
