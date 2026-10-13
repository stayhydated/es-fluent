# Use country, currency, and measurement catalogs

Use `iso-types` for typed country selectors, delivery subdivisions, accepted
currencies, and common measurement units. Catalog values carry canonical codes
and English labels; `#[iso_enum]` declares an application-specific subset.

## Choose catalog features

The default catalogs include countries and active tender currencies. Enable
subdivision and measurement features for those workflows:

```toml
[dependencies]
iso-types = { git = "https://github.com/stayhydated/iso-types", features = ["fluent", "3166-2", "units", "unit-conversion"] }
```

`3166-2` supplies states, provinces, territories, districts, and other ISO
subdivision categories. `units` supplies 87 common CLDR identifiers across
16 categories; `unit-conversion` adds ICU4X conversion support.

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

## Display catalog labels

Enable `fluent` and pass a catalog value or declared subset to a manager's message
localization API. The ISO utility module supplies English labels while the
application's selected locale changes. It follows the same label authority as
`english_name()` and English `Display`.

ICU4X owns country and currency names. Subdivision names use owned English FTL.
Measurement display names come directly from pinned CLDR English data; `meter`
displays as `meters`. Unit symbols and quantity formatting belong to localized
formatting. Persist the canonical code returned by `code()`.
