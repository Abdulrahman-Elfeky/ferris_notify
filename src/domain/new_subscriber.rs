use super::{subscriber_email::SubscriberEmail, SubscriberName};

pub struct NewSubscriber {
    pub email: SubscriberEmail,
    pub name: SubscriberName,
}

impl NewSubscriber {
    pub fn parse(email: String, name: String) -> Result<Self, String> {
        match (SubscriberEmail::parse(email), SubscriberName::parse(name)) {
            (Ok(email), Ok(name)) => Ok(Self { email, name }),
            (Ok(_), Err(e)) => Err(e),
            (Err(e), Ok(_)) => Err(e),
            (Err(e1), Err(e2)) => Err(format!("{e1} \n {e2}")),
        }
    }
}
