use std::{env, time::Duration};

use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use sqlx::postgres::{PgConnectOptions, PgSslMode};

use crate::{
    domain::{SubscriberEmail, SubscriberName},
    email_client::EmailClient,
};

#[derive(Deserialize, Clone)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
    pub email_client: EmailClientSetting,
}

#[derive(Deserialize, Clone)]
pub struct ApplicationSettings {
    pub port: u16,
    pub host: String,
    pub base_url: String,
    pub hmac_secret: SecretString,
    pub redis_uri: SecretString,
}

#[derive(Deserialize, Debug, Clone)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: SecretString,
    pub port: u16,
    pub host: String,
    pub database_name: String,
    pub require_ssl: bool,
}

impl DatabaseSettings {
    pub fn connect_options(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };
        PgConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(self.password.expose_secret())
            .port(self.port)
            .ssl_mode(ssl_mode)
            .database(&self.database_name)
    }
}

#[derive(Deserialize, Clone)]
pub struct EmailClientSetting {
    pub base_url: String,
    pub sender_email: String,
    pub sender_name: String,
    pub authorization_token: SecretString,
    pub timeout_milliseconds: u64,
}

impl EmailClientSetting {
    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_milliseconds)
    }

    pub fn sender(&self) -> Result<SubscriberEmail, String> {
        SubscriberEmail::parse(self.sender_email.clone())
    }

    pub fn name(&self) -> Result<SubscriberName, String> {
        SubscriberName::parse(self.sender_name.clone())
    }

    pub fn client(self) -> EmailClient {
        let sender_email = self.sender().expect("Invalid sender email");
        let sender_name = self.name().expect("Invalid sender name");
        let timeout = self.timeout();
        EmailClient::new(
            self.base_url,
            sender_email,
            sender_name,
            self.authorization_token,
            timeout,
        )
    }
}

pub fn get_configurations() -> Result<Settings, config::ConfigError> {
    let base_directory = env::current_dir().expect("Failed to determine the current directory.");
    let configuration_directory = base_directory.join("configuration");

    let environment: Environment = env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT");

    let environment_filename = format!("{}.yaml", environment.as_str());

    let settings = config::Config::builder()
        .add_source(config::File::from(
            configuration_directory.join("base.yaml"),
        ))
        .add_source(config::File::from(
            configuration_directory.join(environment_filename),
        ))
        .build()?;

    settings.try_deserialize::<Settings>()
}

impl DatabaseSettings {
    pub fn get_connection_string(&self) -> SecretString {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.database_name
        )
        .into()
    }
    pub fn get_connection_string_without_db(&self) -> SecretString {
        format!(
            "postgres://{}:{}@{}:{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port
        )
        .into()
    }
}

pub enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{} is not a supported environment. \
            Use either 'local' or 'production'.",
                other
            )),
        }
    }
}
