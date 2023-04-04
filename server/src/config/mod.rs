mod daemon;
mod database;
mod logging;
mod server;
mod wallet;

use std::fs::File;

use anyhow::Result;
pub use daemon::{DaemonConfig, DaemonLoginConfig};
pub use database::DatabaseConfig;
pub use logging::LoggingConfig;
use serde::Deserialize;
pub use server::{ServerConfig, TlsConfig};
pub use wallet::WalletConfig;

pub fn read_config() -> Result<Config> {
    let config_file = File::open("acceptxmr.yaml")?;
    Ok(serde_yaml::from_reader(config_file)?)
}

/// AcceptXMR-Server configuration.
#[derive(Deserialize, PartialEq, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// Config for the client-facing API.
    pub external_api: ServerConfig,
    /// Config for the internal API.
    pub internal_api: ServerConfig,
    /// Monero wallet configuration.
    pub wallet: WalletConfig,
    /// Monero daemon configuration.
    pub daemon: DaemonConfig,
    /// Invoice database configuration.
    pub database: DatabaseConfig,
    /// Logging configuration.
    pub logging: LoggingConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            external_api: ServerConfig::default(),
            internal_api: ServerConfig {
                port: 8081,
                ..Default::default()
            },
            wallet: WalletConfig::default(),
            daemon: DaemonConfig::default(),
            database: DatabaseConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

#[cfg(test)]
mod test {
    use std::{
        net::{Ipv4Addr, Ipv6Addr},
        path::PathBuf,
        str::FromStr,
    };

    use actix_web::http::Uri;
    use log::LevelFilter;
    use monero::{Address, PrivateKey};
    use secrecy::Secret;

    use super::{
        Config, DaemonConfig, DaemonLoginConfig, LoggingConfig, ServerConfig, TlsConfig,
        WalletConfig,
    };
    use crate::config::DatabaseConfig;

    #[test]
    fn test_default() {
        let config = Config::default();

        let expected_config = Config {
            external_api: ServerConfig {
                port: 8080,
                ipv4: Ipv4Addr::LOCALHOST,
                ipv6: Some(Ipv6Addr::LOCALHOST),
                token: None,
                tls: None,
            },
            internal_api: ServerConfig {
                port: 8081,
                ipv4: Ipv4Addr::LOCALHOST,
                ipv6: Some(Ipv6Addr::LOCALHOST),
                token: None,
                tls: None,
            },
            wallet: WalletConfig {
                primary_address: Address::from_str("4613YiHLM6JMH4zejMB2zJY5TwQCxL8p65ufw8kBP5yxX9itmuGLqp1dS4tkVoTxjyH3aYhYNrtGHbQzJQP5bFus3KHVdmf").unwrap(),
                private_viewkey: Secret::new(PrivateKey::from_str("ad2093a5705b9f33e6f0f0c1bc1f5f639c756cdfc168c8f2ac6127ccbdab3a03").unwrap().to_string()),
            },
            daemon: DaemonConfig {
                url: Uri::from_static("https://xmr-node.cakewallet.com:18081"),
                login: None,
            },
            database: DatabaseConfig {
                path: PathBuf::from_str("AcceptXMR_DB/").unwrap(),
            },
            logging: LoggingConfig {
                verbosity: LevelFilter::Info,
            }
        };

        assert_eq!(config, expected_config);
    }

    #[test]
    fn test_from_yaml() {
        let yaml = include_str!("../../tests/config/config.yaml");

        let expected_config = Config {
            external_api: ServerConfig::default(),
            internal_api: ServerConfig {
                port: 8081,
                ipv4: Ipv4Addr::LOCALHOST,
                ipv6: None,
                token: Some(Secret::new("supersecrettoken".to_string())),
                tls: Some(TlsConfig {
                    cert: PathBuf::from_str("/path/to/cert.pem").unwrap(),
                    key: PathBuf::from_str("/path/to/key.pem").unwrap(),
                }),
            },
            wallet: WalletConfig::default(),
            daemon: DaemonConfig {
                url: Uri::from_static("https://node.example.com:18081"),
                login: Some(DaemonLoginConfig {
                    username: "pinkpanther".to_string(),
                    password: Secret::new("supersecretpassword".to_string()),
                }),
            },
            database: DatabaseConfig {
                path: PathBuf::from_str("server/tests/AcceptXMR_DB/").unwrap(),
            },
            logging: LoggingConfig {
                verbosity: LevelFilter::Debug,
            },
        };

        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config, expected_config);
    }
}
