use actix_web::http::Uri;
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};

#[serde_as]
#[derive(Deserialize, PartialEq, Debug)]
pub struct DaemonConfig {
    /// URL of monero daemon.
    #[serde_as(as = "DisplayFromStr")]
    pub url: Uri,
    /// Monero daemon login credentials, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login: Option<DaemonLoginConfig>,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            url: Uri::from_static("https://xmr-node.cakewallet.com:18081"),
            login: None,
        }
    }
}

/// Username and password of monero daemon.
#[derive(Deserialize, Debug)]
pub struct DaemonLoginConfig {
    pub username: String,
    pub password: Secret<String>,
}

impl PartialEq for DaemonLoginConfig {
    fn eq(&self, other: &Self) -> bool {
        let usernames_match = self.username == other.username;
        let passwords_match = self.password.expose_secret() == other.password.expose_secret();

        usernames_match && passwords_match
    }
}
