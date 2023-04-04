use std::{
    net::{Ipv4Addr, Ipv6Addr},
    path::PathBuf,
};

use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;

// Http(s) server configuration.
#[derive(Deserialize, Debug)]
pub struct ServerConfig {
    /// Port to listen on.
    pub port: u16,
    /// IPv4 to listen on.
    pub ipv4: Ipv4Addr,
    /// IPv6 to listen on.
    pub ipv6: Option<Ipv6Addr>,
    /// Bearer auth token to require. If used, TLS should also be enabled to
    /// prevent exposing the token over the wire.
    ///
    /// It is recommended that secrets like this be set via environment variable
    /// when possible.
    pub token: Option<Secret<String>>,
    /// TLS configuration. Should be enabled if tokens are used.
    pub tls: Option<TlsConfig>,
}

impl PartialEq for ServerConfig {
    fn eq(&self, other: &Self) -> bool {
        let ports_match = self.port == other.port;
        let ipv4s_match = self.ipv4 == other.ipv4;
        let ipv6s_match = self.ipv6 == other.ipv6;
        let tokens_match = self.token.as_ref().map(ExposeSecret::expose_secret)
            == other.token.as_ref().map(ExposeSecret::expose_secret);
        let tls_matches = self.tls == other.tls;

        ports_match && ipv4s_match && ipv6s_match && tokens_match && tls_matches
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            ipv4: Ipv4Addr::LOCALHOST,
            ipv6: Some(Ipv6Addr::LOCALHOST),
            token: None,
            tls: None,
        }
    }
}

#[derive(Deserialize, PartialEq, Eq, Debug)]
pub struct TlsConfig {
    /// Path to TLS certificate `.pem` file.
    pub cert: PathBuf,
    /// Path to TLS certificate `.key` file.
    pub key: PathBuf,
}
