//! Identity domain rules: roles, statuses and validated value objects.
//!
//! Pure logic only. Nothing here talks to a database, HTTP or a hashing library.

use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::{DomainError, DomainResult};

/// Platform-wide role of an account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Role {
    /// Regular participant.
    Player,
    /// Member of a verified organization; runs events.
    OrganizationMember,
    /// Platform administrator.
    Admin,
}

impl Role {
    /// Database/wire representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Player => "PLAYER",
            Self::OrganizationMember => "ORGANIZATION_MEMBER",
            Self::Admin => "ADMIN",
        }
    }

    /// Roles a user may choose at self-registration.
    ///
    /// `ADMIN` is deliberately excluded: privilege is granted, never requested.
    #[must_use]
    pub const fn is_self_assignable(self) -> bool {
        matches!(self, Self::Player | Self::OrganizationMember)
    }

    /// Whether this role satisfies a requirement for `required`.
    ///
    /// `ADMIN` satisfies every requirement; other roles must match exactly.
    #[must_use]
    pub fn satisfies(self, required: Self) -> bool {
        self == required || self == Self::Admin
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Role {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "PLAYER" => Ok(Self::Player),
            "ORGANIZATION_MEMBER" => Ok(Self::OrganizationMember),
            "ADMIN" => Ok(Self::Admin),
            other => Err(DomainError::Validation(format!("unknown role: {other}"))),
        }
    }
}

/// Lifecycle state of an account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UserStatus {
    /// May authenticate normally.
    Active,
    /// Blocked by an administrator.
    Suspended,
    /// Soft deleted.
    Deleted,
}

impl UserStatus {
    /// Database/wire representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "ACTIVE",
            Self::Suspended => "SUSPENDED",
            Self::Deleted => "DELETED",
        }
    }

    /// Whether an account in this state may obtain a session.
    #[must_use]
    pub const fn can_authenticate(self) -> bool {
        matches!(self, Self::Active)
    }
}

impl FromStr for UserStatus {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "ACTIVE" => Ok(Self::Active),
            "SUSPENDED" => Ok(Self::Suspended),
            "DELETED" => Ok(Self::Deleted),
            other => Err(DomainError::Validation(format!("unknown status: {other}"))),
        }
    }
}

/// Review state of an organization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerificationStatus {
    /// Awaiting administrator review.
    Pending,
    /// Approved; may create events.
    Approved,
    /// Rejected by an administrator.
    Rejected,
    /// Previously approved, now blocked.
    Suspended,
}

impl VerificationStatus {
    /// Database/wire representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Approved => "APPROVED",
            Self::Rejected => "REJECTED",
            Self::Suspended => "SUSPENDED",
        }
    }
}

impl FromStr for VerificationStatus {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "PENDING" => Ok(Self::Pending),
            "APPROVED" => Ok(Self::Approved),
            "REJECTED" => Ok(Self::Rejected),
            "SUSPENDED" => Ok(Self::Suspended),
            other => Err(DomainError::Validation(format!(
                "unknown verification status: {other}"
            ))),
        }
    }
}

/// A validated username: 3-32 chars, ASCII alphanumeric plus `_` and `-`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Username(String);

impl Username {
    /// Minimum accepted length.
    pub const MIN_LEN: usize = 3;
    /// Maximum accepted length.
    pub const MAX_LEN: usize = 32;

    /// Validates and normalizes (trims) a username.
    ///
    /// # Errors
    /// Returns [`DomainError::Validation`] when length or charset is violated.
    pub fn parse(raw: &str) -> DomainResult<Self> {
        let trimmed = raw.trim();
        let len = trimmed.chars().count();
        if !(Self::MIN_LEN..=Self::MAX_LEN).contains(&len) {
            return Err(DomainError::Validation(format!(
                "username must be between {} and {} characters",
                Self::MIN_LEN,
                Self::MAX_LEN
            )));
        }
        if !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(DomainError::Validation(
                "username may only contain letters, digits, '_' and '-'".into(),
            ));
        }
        Ok(Self(trimmed.to_string()))
    }

    /// Borrows the validated value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<Username> for String {
    fn from(value: Username) -> Self {
        value.0
    }
}

/// A validated email address, normalized to lowercase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EmailAddress(String);

impl EmailAddress {
    /// Maximum accepted length (RFC 5321 practical limit).
    pub const MAX_LEN: usize = 254;

    /// Validates and lowercases an email address.
    ///
    /// Deliberately a pragmatic structural check rather than a full RFC parser:
    /// real verification happens by sending mail, not by regex.
    ///
    /// # Errors
    /// Returns [`DomainError::Validation`] when the address is not plausible.
    pub fn parse(raw: &str) -> DomainResult<Self> {
        let trimmed = raw.trim();
        let invalid = || DomainError::Validation("email address is not valid".to_string());
        if trimmed.len() > Self::MAX_LEN || trimmed.contains(char::is_whitespace) {
            return Err(invalid());
        }
        let (local, domain) = trimmed.split_once('@').ok_or_else(invalid)?;
        if local.is_empty() || domain.len() < 3 || domain.contains('@') {
            return Err(invalid());
        }
        let (host, tld) = domain.rsplit_once('.').ok_or_else(invalid)?;
        if host.is_empty() || tld.len() < 2 || domain.starts_with('.') {
            return Err(invalid());
        }
        Ok(Self(trimmed.to_ascii_lowercase()))
    }

    /// Borrows the normalized value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<EmailAddress> for String {
    fn from(value: EmailAddress) -> Self {
        value.0
    }
}

/// A plaintext password that passed policy checks.
///
/// Never serialized and never logged; its `Debug` output is redacted.
#[derive(Clone)]
pub struct Password(String);

impl Password {
    /// Minimum accepted length.
    pub const MIN_LEN: usize = 12;
    /// Maximum accepted length; bounded to cap hashing cost.
    pub const MAX_LEN: usize = 128;

    /// Checks a password against policy.
    ///
    /// # Errors
    /// Returns [`DomainError::Validation`] when too short, too long, or lacking
    /// character variety.
    pub fn parse(raw: &str) -> DomainResult<Self> {
        if raw.len() < Self::MIN_LEN {
            return Err(DomainError::Validation(format!(
                "password must be at least {} characters",
                Self::MIN_LEN
            )));
        }
        if raw.len() > Self::MAX_LEN {
            return Err(DomainError::Validation(format!(
                "password must be at most {} characters",
                Self::MAX_LEN
            )));
        }
        let has_letter = raw.chars().any(char::is_alphabetic);
        let has_non_letter = raw.chars().any(|c| !c.is_alphabetic());
        if !(has_letter && has_non_letter) {
            return Err(DomainError::Validation(
                "password must contain letters and at least one digit or symbol".into(),
            ));
        }
        Ok(Self(raw.to_string()))
    }

    /// Borrows the plaintext for hashing or verification.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Password {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Password(***)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_satisfies_every_role() {
        assert!(Role::Admin.satisfies(Role::Player));
        assert!(Role::Admin.satisfies(Role::OrganizationMember));
        assert!(!Role::Player.satisfies(Role::Admin));
        assert!(!Role::Player.satisfies(Role::OrganizationMember));
        assert!(Role::Player.satisfies(Role::Player));
    }

    #[test]
    fn admin_role_cannot_be_self_assigned() {
        assert!(!Role::Admin.is_self_assignable());
        assert!(Role::Player.is_self_assignable());
        assert!(Role::OrganizationMember.is_self_assignable());
    }

    #[test]
    fn roles_round_trip_through_strings() {
        for role in [Role::Player, Role::OrganizationMember, Role::Admin] {
            assert_eq!(role.as_str().parse::<Role>().expect("parse"), role);
        }
        assert!("SUPERUSER".parse::<Role>().is_err());
    }

    #[test]
    fn only_active_users_authenticate() {
        assert!(UserStatus::Active.can_authenticate());
        assert!(!UserStatus::Suspended.can_authenticate());
        assert!(!UserStatus::Deleted.can_authenticate());
    }

    #[test]
    fn usernames_enforce_length_and_charset() {
        assert_eq!(
            Username::parse(" eco_hero ").expect("valid").as_str(),
            "eco_hero"
        );
        assert!(Username::parse("ab").is_err());
        assert!(Username::parse(&"a".repeat(33)).is_err());
        assert!(Username::parse("eco hero").is_err());
        assert!(Username::parse("eco@hero").is_err());
    }

    #[test]
    fn emails_are_normalized_and_validated() {
        assert_eq!(
            EmailAddress::parse(" Player@Example.COM ")
                .expect("valid")
                .as_str(),
            "player@example.com"
        );
        for bad in [
            "no-at-sign",
            "a@b",
            "@example.com",
            "a@example.",
            "a b@c.com",
            "a@@b.com",
        ] {
            assert!(EmailAddress::parse(bad).is_err(), "accepted {bad}");
        }
    }

    #[test]
    fn passwords_enforce_policy_and_stay_redacted() {
        let ok = Password::parse("correct-horse-1").expect("valid");
        assert_eq!(format!("{ok:?}"), "Password(***)");
        assert!(Password::parse("short1!").is_err());
        assert!(Password::parse("onlylettershere").is_err());
        assert!(Password::parse(&"a1".repeat(100)).is_err());
    }
}
