//! PostgreSQL adapter: pool construction, migrations and readiness probe.

use std::time::Duration;

use ecoquest_application::{AppError, AppResult, HealthProbe};
use sqlx::postgres::{PgPool, PgPoolOptions};

/// Thin wrapper over a PostgreSQL connection pool.
#[derive(Clone, Debug)]
pub struct PgStore {
    pool: PgPool,
}

impl PgStore {
    /// Builds the pool without opening a connection; the first query (the
    /// startup migration) is what actually reaches PostgreSQL.
    ///
    /// # Errors
    /// Returns an error when the connection URL cannot be parsed.
    pub fn connect_lazy(database_url: &str, max_connections: u32) -> AppResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .acquire_timeout(Duration::from_secs(5))
            .connect_lazy(database_url)
            .map_err(|e| AppError::Infrastructure(e.to_string()))?;
        Ok(Self { pool })
    }

    /// Borrows the underlying pool for repository implementations.
    #[must_use]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Applies pending migrations from `migrations/`.
    ///
    /// # Errors
    /// Returns an error when a migration fails to apply.
    pub async fn run_migrations(&self) -> AppResult<()> {
        sqlx::migrate!("../../migrations")
            .run(&self.pool)
            .await
            .map_err(|e| AppError::Infrastructure(e.to_string()))
    }
}

#[async_trait::async_trait]
impl HealthProbe for PgStore {
    async fn check(&self) -> AppResult<()> {
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Infrastructure(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_malformed_database_url() {
        assert!(PgStore::connect_lazy("not-a-url", 1).is_err());
    }

    // Pool construction registers a Tokio idle reaper, so a runtime must exist.
    #[tokio::test]
    async fn accepts_well_formed_database_url() {
        assert!(PgStore::connect_lazy("postgres://u:p@localhost:5432/db", 1).is_ok());
    }
}
