//! Data structures crossing the application boundary for authentication.

use chrono::{DateTime, Utc};
use ecoquest_domain::{EmailAddress, Password, Role, UserStatus, Username};
use serde::Serialize;
use uuid::Uuid;

/// A persisted user account.
#[derive(Debug, Clone)]
pub struct User {
    /// Primary key.
    pub id: Uuid,
    /// Display name.
    pub username: String,
    /// Normalized email address.
    pub email: String,
    /// Argon2id PHC string.
    pub password_hash: String,
    /// Platform-wide role.
    pub role: Role,
    /// Optional Solana wallet address.
    pub wallet_address: Option<String>,
    /// Account lifecycle state.
    pub status: UserStatus,
    /// Server-authoritative Eco Point total. Clients may only display it.
    pub eco_points: i32,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp.
    pub updated_at: DateTime<Utc>,
}

/// Public projection of a user; never contains the password hash.
#[derive(Debug, Clone, Serialize)]
pub struct UserProfile {
    /// Primary key.
    pub id: Uuid,
    /// Display name.
    pub username: String,
    /// Normalized email address.
    pub email: String,
    /// Platform-wide role.
    pub role: Role,
    /// Optional Solana wallet address.
    pub wallet_address: Option<String>,
    /// Account lifecycle state.
    pub status: UserStatus,
    /// Server-authoritative Eco Point total for display-only progression.
    pub eco_points: i32,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserProfile {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            role: user.role,
            wallet_address: user.wallet_address,
            status: user.status,
            eco_points: user.eco_points,
            created_at: user.created_at,
        }
    }
}

/// Validated registration input. Every new account is a regular `USER`;
/// other roles are granted, never requested at registration.
#[derive(Debug)]
pub struct RegisterCommand {
    /// Desired username.
    pub username: Username,
    /// Desired email address.
    pub email: EmailAddress,
    /// Plaintext password meeting policy.
    pub password: Password,
}

/// Validated login input.
#[derive(Debug)]
pub struct LoginCommand {
    /// Email address used as the login identifier.
    pub email: EmailAddress,
    /// Plaintext password candidate. Not policy-checked: policy may have changed
    /// since the account was created.
    pub password: String,
}

/// Metadata attached to sessions and audit entries.
#[derive(Debug, Clone, Default)]
pub struct RequestContext {
    /// Client user agent, if present.
    pub user_agent: Option<String>,
    /// Client IP address, if resolvable.
    pub ip_address: Option<std::net::IpAddr>,
}

/// A freshly issued session.
#[derive(Debug)]
pub struct SessionTokens {
    /// Short-lived JWT access token.
    pub access_token: String,
    /// Seconds until the access token expires.
    pub access_expires_in: i64,
    /// Opaque refresh token; only its hash is stored.
    pub refresh_token: String,
    /// Absolute refresh token expiry.
    pub refresh_expires_at: DateTime<Utc>,
}

/// Result of a successful authentication.
#[derive(Debug)]
pub struct AuthOutcome {
    /// The authenticated user.
    pub user: UserProfile,
    /// Tokens for the new session.
    pub tokens: SessionTokens,
}

/// A stored refresh token record.
#[derive(Debug, Clone)]
pub struct RefreshTokenRecord {
    /// Primary key.
    pub id: Uuid,
    /// Owning user.
    pub user_id: Uuid,
    /// Expiry instant.
    pub expires_at: DateTime<Utc>,
    /// Revocation instant, if revoked.
    pub revoked_at: Option<DateTime<Utc>>,
    /// Successor token id once rotated.
    pub replaced_by: Option<Uuid>,
}

impl RefreshTokenRecord {
    /// Whether the token may still be exchanged at `now`.
    #[must_use]
    pub fn is_usable(&self, now: DateTime<Utc>) -> bool {
        self.revoked_at.is_none() && self.replaced_by.is_none() && self.expires_at > now
    }
}

/// An audit trail entry to append.
#[derive(Debug, Clone)]
pub struct AuditEntry {
    /// User responsible, if known.
    pub actor_id: Option<Uuid>,
    /// Action name, e.g. `user.login`.
    pub action: String,
    /// Affected entity type, e.g. `user`.
    pub entity_type: String,
    /// Affected entity id, if any.
    pub entity_id: Option<Uuid>,
    /// Extra structured detail; must never contain secrets.
    pub metadata: serde_json::Value,
    /// Origin IP address.
    pub ip_address: Option<std::net::IpAddr>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record() -> RefreshTokenRecord {
        RefreshTokenRecord {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            expires_at: Utc::now() + chrono::Duration::days(1),
            revoked_at: None,
            replaced_by: None,
        }
    }

    #[test]
    fn fresh_token_is_usable() {
        assert!(record().is_usable(Utc::now()));
    }

    #[test]
    fn expired_revoked_or_rotated_tokens_are_not_usable() {
        let now = Utc::now();

        let mut expired = record();
        expired.expires_at = now - chrono::Duration::seconds(1);
        assert!(!expired.is_usable(now));

        let mut revoked = record();
        revoked.revoked_at = Some(now);
        assert!(!revoked.is_usable(now));

        let mut rotated = record();
        rotated.replaced_by = Some(Uuid::new_v4());
        assert!(!rotated.is_usable(now));
    }
}
