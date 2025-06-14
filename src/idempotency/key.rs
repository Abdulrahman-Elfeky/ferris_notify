pub struct IdempotencyKey(String);

impl TryFrom<String> for IdempotencyKey {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.is_empty() {
            return Err("The idempotency key can't be empty.".into());
        }

        let max_length = 50;
        if s.len() > max_length {
            return Err(format!(
                "The idempotency key must be shorter than {} characters",
                max_length
            ));
        }

        Ok(Self(s))
    }
}

impl From<IdempotencyKey> for String {
    fn from(k: IdempotencyKey) -> Self {
        k.0
    }
}

impl AsRef<str> for IdempotencyKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
