//! Feature-gated example to test CLI with conditional es-fluent derives.

/// A validation error that uses es-fluent when the "fluent" feature is enabled.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "fluent", derive(es_fluent::EsFluent))]
pub struct LenValidation {
    pub min: usize,
    pub max: usize,
    pub actual: usize,
}

/// Another validation error with the same pattern.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "fluent", derive(es_fluent::EsFluent))]
pub struct RangeValidation {
    pub min: i32,
    pub max: i32,
    pub actual: i32,
}

/// An enum with feature-gated derives.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "fluent", derive(es_fluent::EsFluent))]
pub enum ValidationError {
    TooShort { min: usize, actual: usize },
    TooLong { max: usize, actual: usize },
    OutOfRange { min: i32, max: i32, actual: i32 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_len_validation() {
        let v = LenValidation {
            min: 1,
            max: 10,
            actual: 5,
        };
        assert_eq!(v.min, 1);
    }
}
