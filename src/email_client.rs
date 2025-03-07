use std::{sync::Arc, time::Duration};

use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

use crate::{
    configuration::EmailClientSetting,
    domain::{SubscriberEmail, SubscriberName},
};

#[derive(Clone)]
pub struct EmailClient {
    base_url: Arc<String>,
    http_client: Client,
    sender_email: Arc<SubscriberEmail>,
    sender_name: Arc<SubscriberName>,
    authorization_token: Arc<SecretString>,
}

impl EmailClient {
    pub fn new(
        base_url: String,
        sender_email: SubscriberEmail,
        sender_name: SubscriberName,
        authorization_token: SecretString,
        timeout: Duration,
    ) -> Self {
        let base_url = Arc::new(base_url);
        let sender_email = Arc::new(sender_email);
        let sender_name = Arc::new(sender_name);
        let authorization_token = Arc::new(authorization_token);

        Self {
            base_url,
            sender_email,
            sender_name,
            authorization_token,
            http_client: Client::builder().timeout(timeout).build().unwrap(),
        }
    }
    pub async fn send_email(
        &self,
        recipient_email: &SubscriberEmail,
        recipient_name: &SubscriberName,
        subject: &str,
        html_content: &str,
    ) -> Result<(), reqwest::Error> {
        let request = SendEmailRequest {
            sender: Sender {
                email: self.sender_email.as_ref().as_ref().to_owned(),
                name: self.sender_name.as_ref().as_ref().to_owned(),
            },
            to: vec![Recipient {
                email: recipient_email.as_ref().to_owned(),
                name: recipient_name.as_ref().to_owned(),
            }],
            html_content: html_content.to_owned(),
            subject: subject.to_owned(),
        };

        let _ = self
            .http_client
            .post(format!("{}/email", self.base_url.as_ref()))
            .header("api-key", self.authorization_token.as_ref().expose_secret())
            .json(&request)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}

impl TryFrom<EmailClientSetting> for EmailClient {
    type Error = String;

    fn try_from(email_client_setting: EmailClientSetting) -> Result<Self, Self::Error> {
        let timeout = email_client_setting.timeout();
        let sender_email = SubscriberEmail::parse(email_client_setting.sender_email)?;
        let sender_name = SubscriberName::parse(email_client_setting.sender_name)?;

        Ok(Self::new(
            email_client_setting.base_url,
            sender_email,
            sender_name,
            email_client_setting.authorization_token,
            timeout,
        ))
    }
}

#[derive(Serialize, Deserialize)]
struct Sender {
    name: String,
    email: String,
}

#[derive(Serialize, Deserialize)]
struct Recipient {
    email: String,
    name: String,
}

#[derive(Serialize, Deserialize)]
struct SendEmailRequest {
    sender: Sender,
    to: Vec<Recipient>,
    subject: String,
    html_content: String,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use claims::{assert_err, assert_ok};
    use fake::{
        faker::{
            internet::en::SafeEmail,
            lorem::en::{Paragraph, Sentence},
        },
        Fake, Faker,
    };
    use wiremock::{
        matchers::{header, header_exists, method, path},
        Match, Mock, MockServer, ResponseTemplate,
    };

    use super::*;

    struct SendEmailBodyMatcher;

    impl Match for SendEmailBodyMatcher {
        fn matches(&self, request: &wiremock::Request) -> bool {
            if let Ok(_) = request.body_json::<SendEmailRequest>() {
                true
            } else {
                false
            }
        }
    }

    fn subject() -> String {
        Sentence(1..2).fake()
    }

    fn content() -> String {
        Paragraph(1..10).fake()
    }

    fn email() -> SubscriberEmail {
        SubscriberEmail::parse(SafeEmail().fake()).unwrap()
    }

    fn name() -> SubscriberName {
        SubscriberName::parse("Abdulrahman".into()).unwrap()
    }

    fn email_client(base_url: String) -> EmailClient {
        EmailClient::new(
            base_url,
            email(),
            name(),
            SecretString::from(Faker.fake::<String>()),
            Duration::from_millis(200),
        )
    }

    #[tokio::test]
    async fn send_email_sends_the_expected_request() {
        let server = MockServer::start().await;

        Mock::given(header_exists("api-key"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;

        let email_client = email_client(server.uri());

        let outcome = email_client
            .send_email(&email(), &name(), &subject(), &content())
            .await;

        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn send_email_fails_if_the_server_returns_500() {
        let server = MockServer::start().await;

        Mock::given(header_exists("api-key"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&server)
            .await;

        let email_client = email_client(server.uri());

        let outcome = email_client
            .send_email(&email(), &name(), &subject(), &content())
            .await;

        assert_err!(outcome);
    }

    #[tokio::test]
    async fn send_email_times_out_if_the_server_takes_too_long() {
        let server = MockServer::start().await;

        Mock::given(header_exists("api-key"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(360)))
            .expect(1)
            .mount(&server)
            .await;

        let email_client = email_client(server.uri());

        let outcome = email_client
            .send_email(&email(), &name(), &subject(), &content())
            .await;

        assert_err!(outcome);
    }
}
