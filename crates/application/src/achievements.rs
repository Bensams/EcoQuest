//! Server-authoritative achievement eligibility and asynchronous minting.

use std::sync::Arc;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{DateTime, Duration, Utc};
use ecoquest_domain::DomainError;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use rand::RngCore;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{AppError, AppResult};

pub const OCEAN_GUARDIAN: &str = "OCEAN_GUARDIAN";
pub const MAX_MINT_ATTEMPTS: i32 = 5;

/// Where an achievement lives. Platform achievements are derived from the
/// `achievement_progress` counters; on-chain ones are minted asynchronously.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum AchievementKind {
    Platform,
    Onchain,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Achievement {
    pub achievement_key: String,
    pub title: String,
    pub description: String,
    pub kind: AchievementKind,
    /// `EARNED`/`IN_PROGRESS` for platform achievements; the mint status for
    /// on-chain ones.
    pub status: String,
    pub progress: i32,
    pub threshold: i32,
    pub earned: bool,
    /// Opaque on-chain reference; absent for platform achievements.
    pub verification_reference: Option<String>,
    pub wallet_address: Option<String>,
    pub mint_identifier: Option<String>,
    pub transaction_signature: Option<String>,
    pub explorer_url: Option<String>,
}
#[derive(Debug, Clone)]
pub struct WalletChallenge {
    pub nonce: String,
    pub message: String,
    pub expires_at: DateTime<Utc>,
}
#[derive(Debug, Clone)]
pub struct MintJob {
    pub achievement_id: Uuid,
    pub verification_reference: String,
    pub wallet_address: String,
    pub attempts: i32,
}
#[derive(Debug, Clone)]
pub struct MintResult {
    pub mint_identifier: String,
    pub transaction_signature: String,
}

#[async_trait::async_trait]
pub trait BlockchainAdapter: Send + Sync {
    async fn mint(&self, job: &MintJob) -> AppResult<MintResult>;
    fn explorer_url(&self, signature: &str) -> Option<String>;
}
/// Deterministic test adapter. Never submit its result to a real network.
#[derive(Default)]
pub struct MockBlockchainAdapter;
#[async_trait::async_trait]
impl BlockchainAdapter for MockBlockchainAdapter {
    async fn mint(&self, job: &MintJob) -> AppResult<MintResult> {
        let id = hex(&Sha256::digest(job.verification_reference.as_bytes()));
        Ok(MintResult {
            mint_identifier: format!("mock-mint-{id}"),
            transaction_signature: format!("mock-tx-{id}"),
        })
    }
    fn explorer_url(&self, signature: &str) -> Option<String> {
        Some(format!("mock://chain/tx/{signature}"))
    }
}

#[async_trait::async_trait]
pub trait AchievementStore: Send + Sync {
    async fn create_wallet_challenge(
        &self,
        user_id: Uuid,
        wallet: &str,
        nonce_hash: &[u8],
        expires_at: DateTime<Utc>,
    ) -> AppResult<()>;
    async fn consume_wallet_challenge(
        &self,
        user_id: Uuid,
        wallet: &str,
        nonce_hash: &[u8],
        now: DateTime<Utc>,
    ) -> AppResult<bool>;
    async fn set_wallet(&self, user_id: Uuid, wallet: &str) -> AppResult<()>;
    async fn list_for_user(&self, user_id: Uuid) -> AppResult<Vec<Achievement>>;
    async fn queue_eligible_for_wallet(&self, user_id: Uuid, wallet: &str) -> AppResult<()>;
    async fn claim_mint_job(&self, now: DateTime<Utc>) -> AppResult<Option<MintJob>>;
    async fn minted(&self, id: Uuid, result: &MintResult) -> AppResult<()>;
    async fn failed(
        &self,
        id: Uuid,
        attempts: i32,
        retry_at: Option<DateTime<Utc>>,
        error: &str,
    ) -> AppResult<()>;
}

#[derive(Clone)]
pub struct AchievementService {
    store: Arc<dyn AchievementStore>,
    chain: Arc<dyn BlockchainAdapter>,
}
impl AchievementService {
    #[must_use]
    pub fn new(store: Arc<dyn AchievementStore>, chain: Arc<dyn BlockchainAdapter>) -> Self {
        Self { store, chain }
    }
    pub async fn challenge(&self, user: Uuid, wallet: &str) -> AppResult<WalletChallenge> {
        wallet_public_key(wallet)?;
        let mut bytes = [0_u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        let nonce = STANDARD.encode(bytes);
        let expires_at = Utc::now() + Duration::minutes(5);
        self.store
            .create_wallet_challenge(user, wallet, &Sha256::digest(nonce.as_bytes()), expires_at)
            .await?;
        Ok(WalletChallenge {
            message: message(wallet, &nonce),
            nonce,
            expires_at,
        })
    }
    pub async fn verify_wallet(
        &self,
        user: Uuid,
        wallet: &str,
        nonce: &str,
        signature: &str,
    ) -> AppResult<()> {
        let (key_bytes, chain) = wallet_public_key(wallet)?;
        let nonce_hash = Sha256::digest(nonce.as_bytes());
        if !self
            .store
            .consume_wallet_challenge(user, wallet, &nonce_hash, Utc::now())
            .await?
        {
            return Err(AppError::Domain(DomainError::Forbidden(
                "wallet challenge is invalid or expired".into(),
            )));
        }
        let key = VerifyingKey::from_bytes(&key_bytes).map_err(|_| invalid_wallet())?;
        let sig = Signature::from_slice(&STANDARD.decode(signature).map_err(|_| {
            AppError::Domain(DomainError::Validation("invalid wallet signature".into()))
        })?)
        .map_err(|_| {
            AppError::Domain(DomainError::Validation("invalid wallet signature".into()))
        })?;
        key.verify(&signing_payload(chain, &message(wallet, nonce)), &sig)
            .map_err(|_| {
                AppError::Domain(DomainError::Forbidden(
                    "wallet signature does not match address".into(),
                ))
            })?;
        self.store.set_wallet(user, wallet).await?;
        self.store.queue_eligible_for_wallet(user, wallet).await
    }
    /// Every achievement for a user: the platform catalogue with live progress,
    /// plus any on-chain achievement they have become eligible for.
    pub async fn mine(&self, user: Uuid) -> AppResult<Vec<Achievement>> {
        let mut achievements = self.store.list_for_user(user).await?;
        for achievement in &mut achievements {
            // The explorer link belongs to the chain adapter, not the database.
            achievement.explorer_url = achievement
                .transaction_signature
                .as_deref()
                .and_then(|signature| self.chain.explorer_url(signature));
        }
        Ok(achievements)
    }
    /// Claims at most one durable outbox job. Verification ledger is never mutated here.
    pub async fn process_one(&self) -> AppResult<bool> {
        let Some(job) = self.store.claim_mint_job(Utc::now()).await? else {
            return Ok(false);
        };
        match self.chain.mint(&job).await {
            Ok(result) => self.store.minted(job.achievement_id, &result).await?,
            Err(error) => {
                let attempts = job.attempts + 1;
                let retry = (attempts < MAX_MINT_ATTEMPTS)
                    .then(|| Utc::now() + Duration::minutes(1_i64 << (attempts - 1)));
                self.store
                    .failed(job.achievement_id, attempts, retry, &error.to_string())
                    .await?;
            }
        }
        Ok(true)
    }
}
fn message(wallet: &str, nonce: &str) -> String {
    format!("EcoQuest wallet ownership\nwallet:{wallet}\nnonce:{nonce}")
}

/// SEP-53 domain separator. Stellar wallets prepend it so a signed message can
/// never be replayed as a transaction.
const SEP53_PREFIX: &[u8] = b"Stellar Signed Message:\n";

/// The bytes a wallet on `chain` actually puts through Ed25519, which is not
/// the same on both chains.
///
/// Stellar wallets implement SEP-53: they sign `SHA-256(prefix || message)`,
/// never the message itself. Solana's `signMessage` signs the raw message
/// bytes. Verifying the raw bytes for both makes every Freighter link fail
/// with a signature mismatch.
fn signing_payload(chain: Chain, message: &str) -> Vec<u8> {
    match chain {
        Chain::Solana => message.as_bytes().to_vec(),
        Chain::Stellar => Sha256::new()
            .chain_update(SEP53_PREFIX)
            .chain_update(message.as_bytes())
            .finalize()
            .to_vec(),
    }
}
fn invalid_wallet() -> AppError {
    AppError::Domain(DomainError::Validation(
        "invalid wallet address: expected a Solana base58 or Stellar 'G...' address".into(),
    ))
}

/// Chain inferred from the address encoding. Both chains sign with Ed25519.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chain {
    Solana,
    Stellar,
}

impl Chain {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Solana => "solana",
            Self::Stellar => "stellar",
        }
    }
}

/// Decodes a wallet address to its Ed25519 public key, detecting the chain by encoding.
///
/// Stellar account IDs are StrKey: base32 of `version || key || CRC16`, always
/// 56 characters starting with `G`. Everything else is treated as Solana base58.
///
/// # Errors
/// Returns a validation error when the address is not a well-formed 32-byte key.
pub fn wallet_public_key(wallet: &str) -> AppResult<([u8; 32], Chain)> {
    if wallet.len() == 56 && wallet.starts_with('G') {
        return Ok((
            decode_strkey(wallet).ok_or_else(invalid_wallet)?,
            Chain::Stellar,
        ));
    }
    let key: [u8; 32] = bs58::decode(wallet)
        .into_vec()
        .map_err(|_| invalid_wallet())?
        .try_into()
        .map_err(|_| invalid_wallet())?;
    Ok((key, Chain::Solana))
}

/// Base32 (RFC 4648, unpadded) StrKey decode with checksum verification.
fn decode_strkey(address: &str) -> Option<[u8; 32]> {
    const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut bytes = Vec::with_capacity(35);
    let (mut buffer, mut bits) = (0_u32, 0_u32);
    for symbol in address.bytes() {
        let value = ALPHABET.iter().position(|c| *c == symbol)? as u32;
        buffer = (buffer << 5) | value;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            bytes.push((buffer >> bits) as u8);
        }
    }
    // 56 base32 symbols carry 280 bits; the trailing 0 bits must be unset.
    if bytes.len() != 35 || buffer & ((1 << bits) - 1) != 0 {
        return None;
    }
    // Version 6 << 3 marks an ed25519 account id.
    if bytes[0] != 0x30 {
        return None;
    }
    let expected = u16::from_le_bytes([bytes[33], bytes[34]]);
    if crc16_xmodem(&bytes[..33]) != expected {
        return None;
    }
    bytes[1..33].try_into().ok()
}

/// CRC16-XModem (poly 0x1021, init 0), the checksum Stellar StrKey uses.
fn crc16_xmodem(data: &[u8]) -> u16 {
    let mut crc = 0_u16;
    for byte in data {
        crc ^= u16::from(*byte) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn mock_proof_has_no_personal_data() {
        let job = MintJob {
            achievement_id: Uuid::new_v4(),
            verification_reference: "opaque-reference".into(),
            wallet_address: "wallet".into(),
            attempts: 0,
        };
        let result = MockBlockchainAdapter.mint(&job).await.unwrap();
        assert!(!result.mint_identifier.contains('@'));
        assert!(!result.transaction_signature.contains("location"));
    }
    #[test]
    fn retry_is_bounded() {
        assert_eq!(MAX_MINT_ATTEMPTS, 5);
    }

    /// Standard CRC16/XMODEM check value, so the checksum is not merely self-consistent.
    #[test]
    fn crc16_matches_published_check_value() {
        assert_eq!(crc16_xmodem(b"123456789"), 0x31C3);
    }

    fn encode_strkey(key: &[u8; 32]) -> String {
        const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
        let mut payload = Vec::with_capacity(35);
        payload.push(0x30);
        payload.extend_from_slice(key);
        payload.extend_from_slice(&crc16_xmodem(&payload).to_le_bytes());
        let (mut out, mut buffer, mut bits) = (String::new(), 0_u32, 0_u32);
        for byte in payload {
            buffer = (buffer << 8) | u32::from(byte);
            bits += 8;
            while bits >= 5 {
                bits -= 5;
                out.push(ALPHABET[((buffer >> bits) & 31) as usize] as char);
            }
        }
        if bits > 0 {
            out.push(ALPHABET[((buffer << (5 - bits)) & 31) as usize] as char);
        }
        out
    }

    #[test]
    fn stellar_and_solana_addresses_are_both_accepted() {
        let key = [7_u8; 32];
        let stellar = encode_strkey(&key);
        assert_eq!(stellar.len(), 56);
        assert_eq!(wallet_public_key(&stellar).unwrap(), (key, Chain::Stellar));
        let solana = bs58::encode(key).into_string();
        assert_eq!(wallet_public_key(&solana).unwrap(), (key, Chain::Solana));
    }

    /// Published SEP-53 test vector (ecosystem/sep-0053.md, "Sign a simple
    /// ASCII message"). Signing the raw message instead of the prefixed hash
    /// is exactly the mistake that made every Freighter link fail, so this
    /// pins the payload against the spec rather than against ourselves.
    #[test]
    fn stellar_payload_matches_the_sep53_test_vector() {
        const ADDRESS: &str = "GBXFXNDLV4LSWA4VB7YIL5GBD7BVNR22SGBTDKMO2SBZZHDXSKZYCP7L";
        const SIGNATURE: &str = "fO5dbYhXUhBMhe6kId/cuVq/AfEnHRHEvsP8vXh03M1uLpi5e46yO2Q8rEBzu3feXQewcQE5GArp88u6ePK6BA==";
        let (key, chain) = wallet_public_key(ADDRESS).unwrap();
        assert_eq!(chain, Chain::Stellar);
        let key = VerifyingKey::from_bytes(&key).unwrap();
        let sig = Signature::from_slice(&STANDARD.decode(SIGNATURE).unwrap()).unwrap();

        assert!(
            key.verify(&signing_payload(Chain::Stellar, "Hello, World!"), &sig)
                .is_ok(),
            "SEP-53 payload rejected a signature the spec says is valid"
        );
        // The pre-fix behaviour: raw message bytes must not verify.
        assert!(
            key.verify(b"Hello, World!", &sig).is_err(),
            "raw message bytes must not verify a SEP-53 signature"
        );
    }

    /// Solana wallets sign the message itself, so the two chains must not
    /// share a payload.
    #[test]
    fn solana_signs_the_raw_message() {
        assert_eq!(signing_payload(Chain::Solana, "hi"), b"hi".to_vec());
        assert_ne!(
            signing_payload(Chain::Stellar, "hi"),
            signing_payload(Chain::Solana, "hi")
        );
    }

    #[test]
    fn corrupted_or_foreign_addresses_are_rejected() {
        let mut tampered: Vec<char> = encode_strkey(&[7_u8; 32]).chars().collect();
        tampered[10] = if tampered[10] == 'A' { 'B' } else { 'A' };
        let tampered: String = tampered.into_iter().collect();
        assert!(
            wallet_public_key(&tampered).is_err(),
            "bad checksum accepted"
        );
        // Muxed accounts (M...) and secret seeds (S...) must never authenticate.
        for bad in ["", "G", &"M".repeat(56), &"S".repeat(56), "not-base58-0OIl"] {
            assert!(wallet_public_key(bad).is_err(), "accepted {bad}");
        }
    }
}
