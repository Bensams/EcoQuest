//! Idempotent development seeding: one administrator plus test accounts.

use ecoquest_application::{
    auth::{ports::AuthStore, PasswordHasherService},
    AppError, AppResult,
};
use ecoquest_domain::{DomainError, EmailAddress, Password, Role, Username};
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
pub const TEST_ACCOUNTS: [SeedAccount; 2] = [
    SeedAccount {
        username: "eco_player",
        email: "player@ecoquest.test",
        role: Role::Player,
    },
    SeedAccount {
        username: "eco_organizer",
        email: "organizer@ecoquest.test",
        role: Role::OrganizationMember,
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
    let password = Password::parse(password).map_err(AppError::Domain)?;

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
        password.expose(),
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
            password.expose(),
            &mut report,
        )
        .await?;
    }

    Ok(report)
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
