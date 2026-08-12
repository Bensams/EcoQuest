//! Environment driven configuration.

use std::{env, net::SocketAddr};

/// Runtime configuration read from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    /// Address the HTTP server binds to.
    pub bind_addr: SocketAddr,
    /// PostgreSQL connection URL.
    pub database_url: String,
    /// Maximum pooled PostgreSQL connections.
    pub database_max_connections: u32,
    /// Exact origins allowed by CORS.
    pub cors_allowed_origins: Vec<String>,
    /// `tracing` filter directive, e.g. `info,ecoquest_api=debug`.
    pub log_filter: String,
    /// Emit JSON logs instead of human readable ones.
    pub log_json: bool,
    /// Secret signing access tokens; at least 32 bytes.
    pub jwt_secret: String,
    /// Access-token lifetime in seconds.
    pub access_token_ttl_secs: i64,
    /// Refresh-token lifetime in seconds.
    pub refresh_token_ttl_secs: i64,
    /// Set the `Secure` cookie attribute. Only disable for local plain HTTP.
    pub cookies_secure: bool,
    /// Authentication attempts allowed per client per window.
    pub auth_rate_limit_max: u32,
    /// Rate-limit window length in seconds.
    pub auth_rate_limit_window_secs: u64,
}

/// Configuration failures.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// A required variable is absent or empty.
    #[error("missing required environment variable: {0}")]
    Missing(&'static str),
    /// A variable is present but unusable.
    #[error("invalid value for {name}: {detail}")]
    Invalid {
        /// Variable name.
        name: &'static str,
        /// Why it was rejected.
        detail: String,
    },
}

impl Config {
    /// Loads configuration from the process environment.
    ///
    /// # Errors
    /// Returns [`ConfigError`] when a required variable is missing or malformed.
    pub fn from_env() -> Result<Self, ConfigError> {
        let database_url = required("DATABASE_URL")?;
        let bind_addr = optional("API_BIND_ADDR")
            .unwrap_or_else(|| "0.0.0.0:8080".to_string())
            .parse()
            .map_err(|e: std::net::AddrParseError| ConfigError::Invalid {
                name: "API_BIND_ADDR",
                detail: e.to_string(),
            })?;
        let database_max_connections = optional("DATABASE_MAX_CONNECTIONS")
            .map_or(Ok(10), |v| v.parse())
            .map_err(|e: std::num::ParseIntError| ConfigError::Invalid {
                name: "DATABASE_MAX_CONNECTIONS",
                detail: e.to_string(),
            })?;
        let cors_allowed_origins = optional("CORS_ALLOWED_ORIGINS")
            .unwrap_or_else(|| "http://localhost:5173".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let jwt_secret = required("JWT_SECRET")?;
        if jwt_secret.len() < 32 {
            return Err(ConfigError::Invalid {
                name: "JWT_SECRET",
                detail: "must be at least 32 characters".to_string(),
            });
        }

        Ok(Self {
            bind_addr,
            database_url,
            database_max_connections,
            cors_allowed_origins,
            log_filter: optional("RUST_LOG").unwrap_or_else(|| "info".to_string()),
            log_json: optional("LOG_JSON").is_some_and(|v| v == "1" || v == "true"),
            jwt_secret,
            access_token_ttl_secs: parse_num("ACCESS_TOKEN_TTL_SECS", 900)?,
            refresh_token_ttl_secs: parse_num("REFRESH_TOKEN_TTL_SECS", 2_592_000)?,
            // Defaults to secure; opting out is an explicit local-dev decision.
            cookies_secure: optional("COOKIES_SECURE").is_none_or(|v| v == "1" || v == "true"),
            auth_rate_limit_max: parse_num("AUTH_RATE_LIMIT_MAX", 10)?,
            auth_rate_limit_window_secs: parse_num("AUTH_RATE_LIMIT_WINDOW_SECS", 60)?,
        })
    }
}

fn optional(name: &'static str) -> Option<String> {
    env::var(name).ok().filter(|v| !v.trim().is_empty())
}

fn required(name: &'static str) -> Result<String, ConfigError> {
    optional(name).ok_or(ConfigError::Missing(name))
}

fn parse_num<T>(name: &'static str, default: T) -> Result<T, ConfigError>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    optional(name).map_or(Ok(default), |v| {
        v.parse::<T>().map_err(|e| ConfigError::Invalid {
            name,
            detail: e.to_string(),
        })
    })
}
