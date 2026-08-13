//! Idempotent development seeding: one administrator plus test accounts, and
//! the full demo dataset of organizations, events and verified impact.

use std::path::Path;

use ecoquest_application::{
    auth::{ports::AuthStore, PasswordHasherService},
    AppError, AppResult,
};
use ecoquest_domain::{DomainError, EmailAddress, Role, Username};
use sqlx::PgPool;

use crate::{PgAuthStore, PgStore};

/// One account to create.
pub struct SeedAccount {
    /// Login name.
    pub username: &'static str,
    /// Login email.
    pub email: &'static str,
    /// Platform role.
    pub role: Role,
}

/// Test accounts created alongside the administrator.
pub const TEST_ACCOUNTS: [SeedAccount; 5] = [
    SeedAccount {
        username: "eco_player",
        email: "player@ecoquest.test",
        role: Role::User,
    },
    SeedAccount {
        username: "eco_organizer",
        email: "organizer@ecoquest.test",
        role: Role::User,
    },
    SeedAccount {
        username: "alex",
        email: "alex@ecoquest.test",
        role: Role::User,
    },
    SeedAccount {
        username: "maria",
        email: "maria@ecoquest.test",
        role: Role::User,
    },
    SeedAccount {
        username: "diego",
        email: "diego@ecoquest.test",
        role: Role::User,
    },
];

/// Report of what seeding did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct SeedReport {
    /// Accounts inserted by this run.
    pub created: Vec<String>,
    /// Accounts that already existed and were left untouched.
    pub skipped: Vec<String>,
}

/// Creates the administrator and test accounts if absent.
///
/// Re-running is safe: existing accounts are never modified, so a seeded
/// password can never silently overwrite a real one.
///
/// Dev convenience: `password` is *not* validated against the production
/// password policy. Seed accounts only exist on a local database, so any
/// password length is accepted here; real registrations still enforce policy.
///
/// # Errors
/// Returns [`AppError::Domain`] when a supplied value fails validation, and
/// [`AppError::Infrastructure`] when the database rejects a write.
pub async fn seed(
    store: &PgStore,
    admin_email: &str,
    admin_username: &str,
    password: &str,
) -> AppResult<SeedReport> {
    let hasher = PasswordHasherService::new();
    let auth = PgAuthStore::new(store);

    let mut report = SeedReport::default();
    // Validate CLI input before writing anything.
    let admin_username = Username::parse(admin_username).map_err(AppError::Domain)?;
    let admin_email = EmailAddress::parse(admin_email).map_err(AppError::Domain)?;

    create_if_missing(
        &auth,
        &hasher,
        admin_username.as_str(),
        admin_email.as_str(),
        Role::Admin,
        password,
        &mut report,
    )
    .await?;

    for account in &TEST_ACCOUNTS {
        let username = Username::parse(account.username).map_err(AppError::Domain)?;
        let email = EmailAddress::parse(account.email).map_err(AppError::Domain)?;
        create_if_missing(
            &auth,
            &hasher,
            username.as_str(),
            email.as_str(),
            account.role,
            password,
            &mut report,
        )
        .await?;
    }

    Ok(report)
}

/// Seeds accounts and the full demo dataset.
///
/// Runs the account seeder first, then executes `sql_path` (the idempotent
/// `scripts/seed-full.sql`) which populates organizations, events,
/// participations, verification ledgers, certificates and achievements.
///
/// Re-running is safe: account creation never overwrites, and the SQL file uses
/// fixed identifiers with `ON CONFLICT DO NOTHING`.
///
/// # Errors
/// Returns [`AppError::Infrastructure`] when the file cannot be read or a
/// statement fails.
pub async fn seed_full(
    store: &PgStore,
    admin_email: &str,
    admin_username: &str,
    password: &str,
    sql_path: &Path,
) -> AppResult<SeedReport> {
    let report = seed(store, admin_email, admin_username, password).await?;
    let sql = std::fs::read_to_string(sql_path).map_err(|e| {
        AppError::Infrastructure(format!("cannot read seed file {}: {e}", sql_path.display()))
    })?;
    // One transaction: the dataset must land atomically or not at all.
    let mut tx = store.pool().begin().await.map_err(db_err)?;
    sqlx::raw_sql(&sql)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
    tx.commit().await.map_err(db_err)?;
    Ok(report)
}

fn db_err(err: sqlx::Error) -> AppError {
    AppError::Infrastructure(err.to_string())
}

async fn create_if_missing(
    auth: &PgAuthStore,
    hasher: &PasswordHasherService,
    username: &str,
    email: &str,
    role: Role,
    password: &str,
    report: &mut SeedReport,
) -> AppResult<()> {
    if auth.find_user_by_email(email).await?.is_some() {
        report.skipped.push(email.to_string());
        return Ok(());
    }
    let hash = hasher.hash(password)?;
    match auth.create_user(username, email, &hash, role).await {
        Ok(_) => {
            report.created.push(email.to_string());
            Ok(())
        }
        // Lost a race with a concurrent seed run; that is still the desired state.
        Err(AppError::Domain(DomainError::Conflict(_))) => {
            report.skipped.push(email.to_string());
            Ok(())
        }
        Err(err) => Err(err),
    }
}

/// Deletes every seeded account. Intended for local resets only.
///
/// # Errors
/// Returns [`AppError::Infrastructure`] when the delete fails.
pub async fn unseed(pool: &PgPool, admin_email: &str) -> AppResult<u64> {
    let emails: Vec<String> = TEST_ACCOUNTS
        .iter()
        .map(|a| a.email.to_string())
        .chain(std::iter::once(admin_email.to_ascii_lowercase()))
        .collect();
    let result = sqlx::query("DELETE FROM users WHERE lower(email) = ANY($1)")
        .bind(&emails)
        .execute(pool)
        .await
        .map_err(|e| AppError::Infrastructure(e.to_string()))?;
    Ok(result.rows_affected())
}
