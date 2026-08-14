# Threat model

## Assets

Passwords, access/refresh tokens, QR codes, participation ledger, point ledger, certificates, organization approval decisions, wallet proofs, and audit logs.

## Trust boundaries and controls

| Threat | Control | Remaining risk |
| --- | --- | --- |
| Password stuffing | Argon2 hashing, generic login failures, per-IP auth limit | In-process limiter is per replica; move counters to Redis before horizontal scale. |
| Session theft / XSS | HttpOnly, Secure outside local development, SameSite=Strict cookies; CSP | SameSite requires same-site frontend/API deployment. Refresh-token reuse revokes every refresh token for that user, but access tokens are stateless: a stolen one stays valid until it expires (`ACCESS_TOKEN_TTL_SECS`, 15 minutes by default). Keep that TTL short, or add a deny list if immediate revocation is required. |
| CSRF | SameSite=Strict cookies, exact CORS origins | Add CSRF tokens if cross-site cookie flow becomes required. |
| QR replay / leakage | 256-bit opaque codes, SHA-256 stored hash, expiration, one active code | Site attendee can share active code; organizer verification remains required. |
| Fake attendance / duplicate rewards | lifecycle checks, one participation per event/user, transactional verification, unique point transaction | Staff verification can be dishonest; audit review required. |
| Impact inflation | contribution is per verified participant, append-only ledger, unique participation metric | Event owner selects declared per-person value; require evidence workflow for high-value impact. |
| Certificate forgery | 256-bit canonical verification hash, uniquely indexed; certificates written once by the outbox worker; `GET /api/certificates/public/{hash}` returns only facts already printed on the certificate | Anyone holding a hash can read that certificate, so treat the hash as the sharing capability. No revocation propagation beyond the `status` column. |
| Wallet impersonation | signed nonce, expiry, consume-once challenge, opaque chain reference | Mock adapter is not a blockchain network. Never market mock mint as on-chain issuance. |
| Data leak in logs/errors | stable error envelope, infra errors hidden, request IDs | Avoid logging request bodies, cookies, tokens, QR codes, wallet signatures. |
| Browser framing/MIME/referrer abuse | CSP, frame deny, nosniff, no-referrer | Review CSP when adding external assets. |

## Operational controls

Use TLS at edge, rotate `JWT_SECRET` with forced session revocation plan, restrict production DB credentials, monitor rate-limit and authorization failures by `X-Request-Id`, test restore quarterly, and review migration/query plans before releases.
