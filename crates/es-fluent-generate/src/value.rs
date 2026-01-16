use heck::ToTitleCase as _;

pub struct ValueFormatter;
impl ValueFormatter {
    pub fn expand(key: &str) -> String {
        let mut parts = key.rsplit('-');
        let last = parts.next().unwrap();
        last.to_title_case()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_formatter_expand() {
        assert_eq!(ValueFormatter::expand("simple-key"), "Key");
        assert_eq!(ValueFormatter::expand("another-test-value"), "Value");
        assert_eq!(ValueFormatter::expand("single"), "Single");
    }
}
