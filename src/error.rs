use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("config directory is unavailable on this system")]
    ConfigDirUnavailable,

    #[error("configuration error: {0}")]
    Config(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("toml deserialize error: {0}")]
    TomlDeserialize(#[from] toml::de::Error),

    #[error("toml serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("url parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("crypto error: {0}")]
    Crypto(String),

    #[error("api request failed ({status}): {message}")]
    ApiStatus { status: u16, message: String },
}

impl From<argon2::Error> for AppError {
    fn from(value: argon2::Error) -> Self {
        Self::Crypto(value.to_string())
    }
}

impl From<chacha20poly1305::aead::Error> for AppError {
    fn from(value: chacha20poly1305::aead::Error) -> Self {
        Self::Crypto(value.to_string())
    }
}
