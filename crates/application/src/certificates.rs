//! Certificate canonicalization. Hashes bind public certificate facts to issued records.

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertificateData {
    pub certificate_number: String,
    pub user_id: Uuid,
    pub event_id: Uuid,
    pub organization_id: Uuid,
    pub issued_at: DateTime<Utc>,
    pub volunteer_duration_minutes: i32,
}

/// Issued certificate as returned to an authorized participant or organizer.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Certificate {
    pub certificate_number: String,
    pub participation_id: Uuid,
    pub user_id: Uuid,
    pub event_id: Uuid,
    pub event_name: String,
    pub organization_name: String,
    pub participant_name: String,
    pub issued_at: DateTime<Utc>,
    pub volunteer_duration_minutes: i32,
    /// Lowercase hex of the canonical hash; also the public verification key.
    pub verification_hash: String,
    pub status: String,
}

/// Organization ownership and approval state for the calling user.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OrganizationStatus {
    pub organization_id: Uuid,
    pub name: String,
    pub verification_status: String,
    pub reviewed_at: Option<DateTime<Utc>>,
}

#[async_trait::async_trait]
pub trait CertificateStore: Send + Sync {
    /// Issues at most one queued `certificate.issue` outbox event. Returns true when one was handled.
    async fn issue_next_queued(&self) -> crate::AppResult<bool>;
    /// Reads a certificate when the requester is the participant or an organizer of the event.
    async fn find_for_requester(
        &self,
        participation_id: Uuid,
        requester_id: Uuid,
    ) -> crate::AppResult<Option<Certificate>>;
    /// Reads a certificate by its public verification hash. No authentication.
    async fn find_by_hash(&self, verification_hash: &[u8])
        -> crate::AppResult<Option<Certificate>>;
    /// Lists organizations the user owns with approval state.
    async fn organization_status(&self, user_id: Uuid)
        -> crate::AppResult<Vec<OrganizationStatus>>;
    /// Lists every certificate issued to the user, newest first.
    async fn list_for_user(&self, user_id: Uuid) -> crate::AppResult<Vec<Certificate>>;
}

/// Certificate issuance and lookup use cases.
#[derive(Clone)]
pub struct CertificateService {
    store: std::sync::Arc<dyn CertificateStore>,
}

impl CertificateService {
    #[must_use]
    pub fn new(store: std::sync::Arc<dyn CertificateStore>) -> Self {
        Self { store }
    }
    pub async fn process_one(&self) -> crate::AppResult<bool> {
        self.store.issue_next_queued().await
    }
    pub async fn for_requester(
        &self,
        participation_id: Uuid,
        requester_id: Uuid,
    ) -> crate::AppResult<Certificate> {
        self.store
            .find_for_requester(participation_id, requester_id)
            .await?
            .ok_or_else(|| {
                crate::AppError::Domain(ecoquest_domain::DomainError::NotFound(
                    "certificate".into(),
                ))
            })
    }
    /// Public verification. `hash_hex` must be 64 lowercase hex characters.
    pub async fn verify_public(&self, hash_hex: &str) -> crate::AppResult<Certificate> {
        let bytes = decode_hash(hash_hex).ok_or_else(|| {
            crate::AppError::Domain(ecoquest_domain::DomainError::Validation(
                "verification hash must be 64 hexadecimal characters".into(),
            ))
        })?;
        self.store.find_by_hash(&bytes).await?.ok_or_else(|| {
            crate::AppError::Domain(ecoquest_domain::DomainError::NotFound("certificate".into()))
        })
    }
    pub async fn organization_status(
        &self,
        user_id: Uuid,
    ) -> crate::AppResult<Vec<OrganizationStatus>> {
        self.store.organization_status(user_id).await
    }
    /// Lists every certificate issued to the user, newest first.
    pub async fn list_for_user(&self, user_id: Uuid) -> crate::AppResult<Vec<Certificate>> {
        self.store.list_for_user(user_id).await
    }
}

/// Parses a 32-byte hash from lowercase hex, rejecting any other shape.
#[must_use]
pub fn decode_hash(hex: &str) -> Option<[u8; 32]> {
    if hex.len() != 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut out = [0u8; 32];
    for (index, slot) in out.iter_mut().enumerate() {
        *slot = u8::from_str_radix(hex.get(index * 2..index * 2 + 2)?, 16).ok()?;
    }
    Some(out)
}

/// Lowercase hex encoding used by public verification URLs.
#[must_use]
pub fn encode_hash(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// SHA-256 over length-prefixed canonical UTF-8 certificate fields.
#[must_use]
pub fn verification_hash(data: &CertificateData) -> [u8; 32] {
    let mut canonical = Vec::new();
    for value in [
        data.certificate_number.as_str(),
        &data.user_id.to_string(),
        &data.event_id.to_string(),
        &data.organization_id.to_string(),
        &data
            .issued_at
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        &data.volunteer_duration_minutes.to_string(),
    ] {
        canonical.extend_from_slice(&(value.len() as u32).to_be_bytes());
        canonical.extend_from_slice(value.as_bytes());
    }
    Sha256::digest(canonical).into()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    /// In-memory [`CertificateStore`] for service-level tests.
    struct FakeStore {
        issued: Vec<Certificate>,
    }

    #[async_trait::async_trait]
    impl CertificateStore for FakeStore {
        async fn issue_next_queued(&self) -> crate::AppResult<bool> {
            Ok(false)
        }
        async fn find_for_requester(
            &self,
            _participation_id: Uuid,
            _requester_id: Uuid,
        ) -> crate::AppResult<Option<Certificate>> {
            Ok(None)
        }
        async fn find_by_hash(
            &self,
            _verification_hash: &[u8],
        ) -> crate::AppResult<Option<Certificate>> {
            Ok(None)
        }
        async fn organization_status(
            &self,
            _user_id: Uuid,
        ) -> crate::AppResult<Vec<OrganizationStatus>> {
            Ok(Vec::new())
        }
        async fn list_for_user(&self, user_id: Uuid) -> crate::AppResult<Vec<Certificate>> {
            let mut owned: Vec<Certificate> = self
                .issued
                .iter()
                .filter(|c| c.user_id == user_id)
                .cloned()
                .collect();
            owned.sort_by_key(|c| std::cmp::Reverse(c.issued_at));
            Ok(owned)
        }
    }

    #[test]
    fn hash_hex_round_trips_and_rejects_malformed_input() {
        let bytes = [0x0au8; 32];
        let hex = encode_hash(&bytes);
        assert_eq!(hex.len(), 64);
        assert_eq!(decode_hash(&hex), Some(bytes));
        for bad in ["", "zz", &"g".repeat(64), &hex[..63]] {
            assert_eq!(decode_hash(bad), None, "accepted {bad}");
        }
    }

    #[test]
    fn tampered_certificate_data_changes_hash() {
        let mut data = CertificateData {
            certificate_number: "ECO-2026-000001".into(),
            user_id: Uuid::nil(),
            event_id: Uuid::nil(),
            organization_id: Uuid::nil(),
            issued_at: DateTime::parse_from_rfc3339("2026-01-02T03:04:05.000Z")
                .expect("date")
                .with_timezone(&Utc),
            volunteer_duration_minutes: 60,
        };
        let hash = verification_hash(&data);
        data.volunteer_duration_minutes = 61;
        assert_ne!(hash, verification_hash(&data));
    }

    #[tokio::test]
    async fn list_for_user_returns_only_owned_certificates_sorted_by_issue_date() {
        let user = Uuid::new_v4();
        let other = Uuid::new_v4();
        let oldest = Certificate {
            certificate_number: "ECO-2026-000001".into(),
            participation_id: Uuid::new_v4(),
            user_id: user,
            event_id: Uuid::new_v4(),
            event_name: "Beach cleanup".into(),
            organization_name: "Green Earth".into(),
            participant_name: "Alex".into(),
            issued_at: Utc::now() - chrono::Duration::days(10),
            volunteer_duration_minutes: 120,
            verification_hash: "a".repeat(64),
            status: "ISSUED".into(),
        };
        let newest = Certificate {
            certificate_number: "ECO-2026-000002".into(),
            participation_id: Uuid::new_v4(),
            user_id: user,
            event_id: Uuid::new_v4(),
            event_name: "Tree planting".into(),
            organization_name: "Green Earth".into(),
            participant_name: "Alex".into(),
            issued_at: Utc::now(),
            volunteer_duration_minutes: 180,
            verification_hash: "b".repeat(64),
            status: "ISSUED".into(),
        };
        let foreign = Certificate {
            certificate_number: "ECO-2026-000003".into(),
            participation_id: Uuid::new_v4(),
            user_id: other,
            event_id: Uuid::new_v4(),
            event_name: "Other cleanup".into(),
            organization_name: "Other org".into(),
            participant_name: "Diego".into(),
            issued_at: Utc::now(),
            volunteer_duration_minutes: 60,
            verification_hash: "c".repeat(64),
            status: "ISSUED".into(),
        };
        let store = FakeStore {
            issued: vec![oldest, newest, foreign],
        };
        let service = CertificateService::new(Arc::new(store));
        let result = service.list_for_user(user).await.expect("list");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].certificate_number, "ECO-2026-000002");
        assert_eq!(result[1].certificate_number, "ECO-2026-000001");
        assert!(result.iter().all(|c| c.user_id == user));
    }
}
