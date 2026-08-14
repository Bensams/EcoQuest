//! Development seeding: creates the bootstrap administrator and test accounts.
//!
//! This lives beside the API rather than in the CLI on purpose. The CLI is
//! API-only and never opens a PostgreSQL connection, but the first `ADMIN`
//! cannot be created over HTTP: `Role::Admin` is not self-assignable at
//! registration. Seeding therefore needs the composition root's database
//! access.
//!
//! Reads `DATABASE_URL`, `SEED_ADMIN_EMAIL`, `SEED_ADMIN_USERNAME` and
//! `SEED_PASSWORD` from the environment (or `.env`). Re-running is safe:
//! existing accounts are left untouched.

use ecoquest_infrastructure::{seed::seed, PgStore};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();

    let database_url = required("DATABASE_URL")?;
    let admin_email = env::var("SEED_ADMIN_EMAIL").unwrap_or_else(|_| "admin@ecoquest.test".into());
    let admin_username = env::var("SEED_ADMIN_USERNAME").unwrap_or_else(|_| "eco_admin".into());
    let password = required("SEED_PASSWORD")?;

    let store = PgStore::connect_lazy(&database_url, 2)?;
    // Seeding a database with no schema yet would fail confusingly.
    store.run_migrations().await?;

    let report = seed(&store, &admin_email, &admin_username, &password).await?;
    for email in &report.created {
        println!("created {email}");
    }
    for email in &report.skipped {
        println!("exists  {email}");
    }
    println!(
        "seed complete: {} created, {} already present",
        report.created.len(),
        report.skipped.len()
    );
    Ok(())
}

fn required(name: &str) -> Result<String, String> {
    env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("missing required environment variable: {name}"))
}
