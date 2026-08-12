//! Structured logging setup.

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Installs the global tracing subscriber.
///
/// Call once at startup; repeated calls are ignored by `tracing`.
pub fn init(filter: &str, json: bool) {
    let env_filter = EnvFilter::try_new(filter).unwrap_or_else(|_| EnvFilter::new("info"));
    let registry = tracing_subscriber::registry().with(env_filter);

    if json {
        registry
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        registry
            .with(tracing_subscriber::fmt::layer().compact())
            .init();
    }
}
