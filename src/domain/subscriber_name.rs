use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct SubscriberName(String);

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl SubscriberName {
    pub fn parse(s: String) -> Result<SubscriberName, String> {
        let is_empty_or_whitespace = s.trim().is_empty();

        let is_too_long = s.graphemes(true).count() > 256;

        let forbidden_characters = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];

        let contains_forbidden_characters = s.contains(forbidden_characters);

        if is_too_long || contains_forbidden_characters || is_empty_or_whitespace {
            Err(format!("{} is not a valid subscriber name.", s))
        } else {
            Ok(SubscriberName(s))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok};

    #[test]
    fn a_256_long_grapheme_is_valid() {
        let name = "ع".repeat(256);
        assert_ok!(SubscriberName::parse(name));
    }

    #[test]
    fn a_257_long_grapheme_is_rejected() {
        let name = "ع".repeat(257);
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn whitespace_only_is_rejected() {
        let name = "    ".to_string();
        assert_err!(SubscriberName::parse(name));
    }
    #[test]
    fn empty_string_is_rejected() {
        let name = "".to_string();
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn troublesome_string_is_rejected() {
        for name in &['/', '(', ')', '"', '<', '>', '\\', '{', '}'] {
            let name = name.to_string();
            assert_err!(SubscriberName::parse(name));
        }
    }

    #[test]
    fn a_valid_name_is_parsed_gracefully() {
        let name = "abdulrahman".to_string();
        assert_ok!(SubscriberName::parse(name));
    }
}
