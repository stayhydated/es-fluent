use heck::ToTitleCase as _;

pub struct ValueFormatter;
impl ValueFormatter {
    pub fn expand(key: &str) -> String {
        let mut parts = key.rsplit('-');
        let last = parts.next().unwrap();
        last.to_title_case()
    }
}
