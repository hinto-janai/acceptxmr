use log::LevelFilter;

use crate::config::LoggingConfig;

pub fn init_logger(config: LoggingConfig) {
    env_logger::builder()
        .filter_level(LevelFilter::Warn)
        .filter_module("acceptxmr", config.verbosity)
        .filter_module("acceptxmr-server", config.verbosity)
        .init();
}
