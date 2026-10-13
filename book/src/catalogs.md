# Use country, measurement, and regional catalogs

Use `iso-types` for typed country selectors, delivery subdivisions, accepted
currencies, common measurement units, and regional metadata. Catalog values carry canonical codes
and English labels; `#[iso_enum]` declares an application-specific subset.

## Choose catalog features

The default catalogs include countries and active tender currencies. Enable
subdivision and measurement features for those workflows:

```toml
[dependencies]
iso-types = { git = "https://github.com/stayhydated/iso-types", features = ["fluent", "3166-2", "units", "unit-conversion"] }
```

`3166-2` supplies states, provinces, territories, districts, and other ISO
subdivision categories. `units` supplies 103 common CLDR identifiers across
16 categories; `unit-conversion` adds ICU4X conversion support.

Add `regions`, `regional-preferences`, `language-relations`, `unit-preferences`,
`time-zones`, `calling-codes`, or `addresses` for the matching country data.
`full` enables all catalogs and regional relationships. `time-zone-localization`
adds ICU time-zone mappings and localized names; `phone-numbers` adds the
`phonenumber` parsing adapter.

## Navigate delivery subdivisions

Start from a country and resolve its case-sensitive local subdivision code:

```rust
use iso_types::{Iso3166_1, Iso3166_2, SubdivisionKind};

assert_eq!(Iso3166_1::Us.subdivision("CA"), Some(Iso3166_2::UsCa));
assert_eq!(Iso3166_2::UsCa.country(), Iso3166_1::Us);
assert_eq!(Iso3166_2::UsCa.kind(), SubdivisionKind::State);
assert_eq!(Iso3166_2::CaQc.kind(), SubdivisionKind::Province);
assert_eq!(Iso3166_2::GbLnd.parent(), Some(Iso3166_2::GbEng));
```

`subdivisions()` lists every current subdivision in a country.
`root_subdivisions()` lists the roots of its hierarchy; `parent()` and
`children()` navigate direct relationships. Use `kind()` to distinguish
administrative categories and `local_code()` for the country-relative suffix.
Lists follow canonical code order.

Select every subdivision in a country without listing codes:

```rust
#[iso_types::iso_enum(standard = "3166-2", country = "US")]
pub enum UsDeliveryAreas {}
```

Use `category = "length"` with `standard = "units"` or `"unit-preferences"`
to select a CLDR category. Filters intersect with an explicit `codes` list and
reject empty selections.

## Select accepted currencies

Declare a subset using canonical ISO 4217 alpha codes:

```rust
use iso_types::{iso_enum, Iso4217, VariantArray as _};

#[iso_enum(standard = "4217", codes = ["USD", "JPY"])]
pub enum AcceptedCurrencies {}

assert_eq!(AcceptedCurrencies::VARIANTS.len(), 2);
assert_eq!(AcceptedCurrencies::Usd.catalog_value(), Iso4217::Usd);
assert_eq!(Iso4217::Jpy.fraction_digits(), 0);
```

Country `currencies()` and currency `countries()` return the active-tender
relationships in the pinned snapshot. These lists support countries with several
currencies and currencies shared by several countries. Their order follows
canonical codes. Countries without active-tender data have an empty list.

`fraction_digits()` and `cash_fraction_digits()` provide ICU4X/CLDR formatting
precision. `rounding_increment()` and `cash_rounding_increment()` express the
increment in the corresponding smallest fraction unit. For example, a cash
increment of `5` with two fraction digits represents `0.05`.

## Convert measurements

Use canonical CLDR core identifiers such as `meter`, `gallon-imperial`, and
`liter-per-100-kilometer`. The catalog covers common metric and customary units,
temperature, speed, fuel economy, digital quantities, and physical quantities.

```rust
use iso_types::{MeasurementUnit, UnitCategory};

assert_eq!(MeasurementUnit::Meter.category(), UnitCategory::Length);
assert_eq!(MeasurementUnit::Meter.code(), "meter");
let centimeters = MeasurementUnit::Meter.convert(1.0, MeasurementUnit::Centimeter)?;
assert_eq!(centimeters, 100.0);
let fahrenheit = MeasurementUnit::Celsius.convert(0.0, MeasurementUnit::Fahrenheit)?;
assert_eq!(fahrenheit, 32.0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

`converter_to::<f64>()` creates a reusable ICU converter. ICU4X supplies factors,
temperature offsets, and reciprocal fuel-economy rules. Incompatible dimensions
return `UnitConversionError` containing the source and target units. Categories
group the catalog; ICU determines conversion compatibility.

Declare a measurement subset with `#[iso_enum(standard = "units", codes =
["meter", "foot"])]`. Call `catalog_value()` to access its category and
conversion methods.

## Query regional defaults and language relationships

Enable `regional-preferences`, `regions`, and `language-relations`:

```rust
use iso_types::{GeographicRegion, Iso3166_1, Iso6391, Iso15924, Iso8601Weekday};

let defaults = Iso3166_1::Us.regional_preferences();
assert_eq!(defaults.week.first_weekday, Iso8601Weekday::Sunday);
assert_eq!(Iso3166_1::Us.geographic_region(), Some(GeographicRegion::NorthernAmerica));
assert!(Iso3166_1::Ca.languages().contains(&Iso6391::Fr));
assert!(Iso6391::Sr.scripts().contains(&Iso15924::Latn));
```

Regional defaults include weekend days, minimum days for the first week, hour
cycle, ordered calendars, measurement systems, and paper size. They reuse ICU4X
preference types; explicit user choices take precedence. `geographic_regions()`
returns the UN M49 hierarchy from World to the most specific recorded region.
Use region `parent()`, `children()`, and `countries()` for navigation.

Language and script associations follow the selected common scope.
`language_usage()` exposes CLDR functional-population estimates, including
second-language users, and its practical language-status definitions.
Script `direction()` delegates to ICU4X.

## Choose units by usage

With `unit-preferences`, ask for a specific measurement usage:

```rust
use iso_types::{Iso3166_1, UnitUsage};

let weather = Iso3166_1::Us.unit_preferences(UnitUsage::TemperatureWeather);
assert_eq!(weather[0].units[0].code(), "fahrenheit");
let height = Iso3166_1::Us.unit_preferences(UnitUsage::LengthPersonHeight);
assert_eq!(height[0].units[0].code(), "foot");
assert_eq!(height[0].units[1].code(), "inch");
```

Preferences retain CLDR's ranking, mixed components, thresholds, and number
skeletons. Thresholds use the largest component; the final choice handles smaller
values. Apply skeleton rounding when selecting units for display. Named units
are available through `catalog_unit()`; `icu_unit()` parses compound components
when `unit-conversion` is enabled.

## Work with time zones

Enable `time-zones` for typed location identifiers and bundled Jiff rules:

```rust
use iso_types::{Iso3166_1, TimeZoneId};

assert!(Iso3166_1::Us.time_zones().contains(&TimeZoneId::AmericaNewYork));
let timestamp = "2026-07-15T12:00Z".parse()?;
let local = TimeZoneId::AmericaNewYork.at(timestamp)?;
assert_eq!(local.offset().seconds(), -4 * 3600);
# Ok::<(), Box<dyn std::error::Error>>(())
```

`time_zone()` exposes Jiff's transition and `to_ambiguous_zoned()` APIs for
explicit DST gap/overlap resolution. With `time-zone-localization`, use
`icu_time_zone()` for ICU identity and `localized_name(&locale)` for location
names in the selected display locale. Persist the canonical IANA code.

## Use telephone and address metadata

Enable `calling-codes` and `addresses` for country-aware inputs:

```rust
use iso_types::{AddressField, AddressFieldLabel, CallingCode, Iso3166_1};

assert_eq!(Iso3166_1::Us.calling_codes(), &[CallingCode::C1]);
assert!(CallingCode::C1.countries().contains(&Iso3166_1::Ca));
let address = Iso3166_1::Us.address_metadata().expect("pinned US address schema");
assert!(address.is_required(AddressField::PostalCode));
assert_eq!(address.field_label(AddressField::AdministrativeArea), AddressFieldLabel::State);
```

Calling-code relationships preserve shared prefixes and global services. With
`phone-numbers`, convert a country into `phonenumber::country::Id`, then delegate
parsing and validation to `phonenumber`.

Address schemas preserve native and Latin templates, field order, required
components, postal patterns/examples, and language-specific label overrides.
`field_label()` returns a typed Fluent label such as State, Province, or Eircode;
`label_override(field, language)` retrieves an exact upstream override. Postal
patterns use Java-compatible regular expressions. The country is supplied
separately from the address components.

## Display catalog labels

Enable `fluent` and pass a catalog value or declared subset to a manager's message
localization API. The ISO utility module supplies English labels while the
application's selected locale changes. It follows the same label authority as
`english_name()` and English `Display`.

ICU4X owns country and currency names. Subdivision names use owned English FTL.
Measurement display names come directly from pinned CLDR English data; `meter`
displays as `meters`. Unit symbols and quantity formatting belong to localized
formatting. Persist the canonical code returned by `code()`.
