//! Access-token issuing/verification and opaque refresh-token generation.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{Duration, Utc};
use ecoquest_domain::{DomainError, Role};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{AppError, AppResult};

/// Claims carried by the access token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessClaims {
    /// Subject: user id.
    pub sub: Uuid,
    /// Platform role at issue time.
    pub role: Role,
    /// Issued-at, seconds since epoch.
    pub iat: i64,
    /// Expiry, seconds since epoch.
    pub exp: i64,
}

/// Issues and verifies HS256 access tokens.
#[derive(Clone)]
pub struct TokenService {
    encoding: EncodingKey,
    decoding: DecodingKey,
    access_ttl: Duration,
    refresh_ttl: Duration,
}

impl TokenService {
    /// Minimum accepted secret length, in bytes.
    pub const MIN_SECRET_LEN: usize = 32;

    /// Builds the service.
    ///
    /// # Errors
    /// Returns [`AppError::Domain`] when the secret is shorter than
    /// [`Self::MIN_SECRET_LEN`], which would make tokens trivially forgeable.
    pub fn new(secret: &[u8], access_ttl_secs: i64, refresh_ttl_secs: i64) -> AppResult<Self> {
        if secret.len() < Self::MIN_SECRET_LEN {
            return Err(AppError::Domain(DomainError::Validation(format!(
                "jwt secret must be at least {} bytes",
                Self::MIN_SECRET_LEN
            ))));
        }
        Ok(Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
            access_ttl: Duration::seconds(access_ttl_secs),
            refresh_ttl: Duration::seconds(refresh_ttl_secs),
        })
    }

    /// Access-token lifetime in seconds.
    #[must_use]
    pub fn access_ttl_secs(&self) -> i64 {
        self.access_ttl.num_seconds()
    }

    /// Refresh-token lifetime in seconds.
    #[must_use]
    pub fn refresh_ttl_secs(&self) -> i64 {
        self.refresh_ttl.num_seconds()
    }

    /// Issues a signed access token for a user.
    ///
    /// # Errors
    /// Returns [`AppError::Infrastructure`] when signing fails.
    pub fn issue_access_token(&self, user_id: Uuid, role: Role) -> AppResult<String> {
        let now = Utc::now();
        let claims = AccessClaims {
            sub: user_id,
            role,
            iat: now.timestamp(),
            exp: (now + self.access_ttl).timestamp(),
        };
        jsonwebtoken::encode(&Header::new(Algorithm::HS256), &claims, &self.encoding)
            .map_err(|e| AppError::Infrastructure(format!("token signing failed: {e}")))
    }

    /// Verifies a token and returns its claims.
    ///
    /// # Errors
    /// Returns [`AppError::Domain`] with [`DomainError::Forbidden`] when the
    /// token is invalid, expired or has a foreign signature.
    pub fn verify_access_token(&self, token: &str) -> AppResult<AccessClaims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.leeway = 0;
        jsonwebtoken::decode::<AccessClaims>(token, &self.decoding, &validation)
            .map(|data| data.claims)
            .map_err(|e| AppError::Domain(DomainError::Forbidden(format!("invalid token: {e}"))))
    }

    /// Generates a 256-bit opaque refresh token and its storage hash.
    #[must_use]
    pub fn generate_refresh_token(&self) -> (String, Vec<u8>) {
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        let token = URL_SAFE_NO_PAD.encode(bytes);
        let hash = hash_refresh_token(&token);
        (token, hash)
    }
}

/// Hashes a refresh token for storage/lookup.
///
/// SHA-256 rather than Argon2: the token is 256 bits of entropy, so brute force
/// is infeasible and lookups must stay a single indexed query.
#[must_use]
pub fn hash_refresh_token(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service() -> TokenService {
        TokenService::new(&[7u8; 32], 900, 2_592_000).expect("service")
    }

    #[test]
    fn rejects_short_secrets() {
        assert!(TokenService::new(b"too-short", 900, 900).is_err());
    }

    #[test]
    fn access_tokens_round_trip() {
        let svc = service();
        let id = Uuid::new_v4();
        let token = svc.issue_access_token(id, Role::Admin).expect("issue");
        let claims = svc.verify_access_token(&token).expect("verify");
        assert_eq!(claims.sub, id);
        assert_eq!(claims.role, Role::Admin);
        assert!(claims.exp > claims.iat);
    }

    #[test]
    fn tokens_signed_with_another_secret_are_rejected() {
        let token = service()
            .issue_access_token(Uuid::new_v4(), Role::User)
            .expect("issue");
        let other = TokenService::new(&[9u8; 32], 900, 900).expect("service");
        assert!(other.verify_access_token(&token).is_err());
    }

    #[test]
    fn expired_access_tokens_are_rejected() {
        let svc = TokenService::new(&[7u8; 32], -1, 900).expect("service");
        let token = svc
            .issue_access_token(Uuid::new_v4(), Role::User)
            .expect("issue");
        assert!(svc.verify_access_token(&token).is_err());
    }

    #[test]
    fn refresh_tokens_are_unique_and_hash_deterministically() {
        let svc = service();
        let (token_a, hash_a) = svc.generate_refresh_token();
        let (token_b, _) = svc.generate_refresh_token();
        assert_ne!(token_a, token_b);
        assert_eq!(hash_a, hash_refresh_token(&token_a));
        assert_ne!(hash_a, hash_refresh_token(&token_b));
        assert_eq!(hash_a.len(), 32);
    }
}
