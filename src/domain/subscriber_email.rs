use std::fmt::Display;

use validator::ValidateEmail;

#[derive(Debug)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(s: String) -> Result<Self, String> {
        if s.validate_email() {
            Ok(Self(s))
        } else {
            Err(format!("{} is not a valid email.", s))
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Display for SubscriberEmail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {

    use claims::{assert_err, assert_ok};
    use fake::{faker::internet::en::SafeEmail, Fake};

    use super::*;

    #[test]
    fn empty_string_is_rejected() {
        let email = "".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "abdulrahmangmail.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@gmail.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[derive(Debug, Clone)]
    struct ValidEmailFixture(String);

    impl quickcheck::Arbitrary for ValidEmailFixture {
        fn arbitrary(_g: &mut quickcheck::Gen) -> Self {
            Self(SafeEmail().fake())
        }
    }

    #[quickcheck_macros::quickcheck]
    fn valid_email_is_parsed_gracefully(email: ValidEmailFixture) {
        //dbg!(&email);
        assert_ok!(SubscriberEmail::parse(email.0));
    }
}
