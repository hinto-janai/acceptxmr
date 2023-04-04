use log::LevelFilter;
use serde::Deserialize;

#[derive(Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub struct LoggingConfig {
    pub verbosity: LevelFilter,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            verbosity: LevelFilter::Info,
        }
    }
}
