//! Error types for brewx-state

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("TOML serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("Config not found: {0}")]
    ConfigNotFound(String),

    #[error("Invalid config: {0}")]
    InvalidConfig(String),
}

pub type Result<T> = std::result::Result<T, Error>;
