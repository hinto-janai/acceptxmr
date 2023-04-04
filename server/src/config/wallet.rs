use std::str::FromStr;

use monero::{Address, PrivateKey};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct WalletConfig {
    /// Monero wallet's primary address. Should begin with a `4`.
    pub primary_address: Address,
    /// Monero wallet private view key. It is good practice to set this secret
    /// via environment variable.
    pub private_viewkey: Secret<String>,
}

impl Default for WalletConfig {
    fn default() -> Self {
        Self {
            primary_address: Address::from_str("4613YiHLM6JMH4zejMB2zJY5TwQCxL8p65ufw8kBP5yxX9itmuGLqp1dS4tkVoTxjyH3aYhYNrtGHbQzJQP5bFus3KHVdmf").unwrap(),
            private_viewkey: Secret::new(PrivateKey::from_str("ad2093a5705b9f33e6f0f0c1bc1f5f639c756cdfc168c8f2ac6127ccbdab3a03").unwrap().to_string()),
        }
    }
}

impl PartialEq for WalletConfig {
    fn eq(&self, other: &Self) -> bool {
        let addresses_match = self.primary_address == other.primary_address;
        let viewkeys_match =
            self.private_viewkey.expose_secret() == other.private_viewkey.expose_secret();

        addresses_match && viewkeys_match
    }
}
