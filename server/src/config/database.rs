use std::{path::PathBuf, str::FromStr};

use serde::Deserialize;

/// Default invoice storage database directory.
const DEFAULT_DB_DIR: &str = "AcceptXMR_DB/";

#[derive(Clone, Deserialize, PartialEq, Eq, Debug)]
pub struct DatabaseConfig {
    pub path: PathBuf,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from_str(DEFAULT_DB_DIR).unwrap(),
        }
    }
}
