# EcoQuest Architecture

## Purpose

EcoQuest is a gamified environmental-impact platform. Verified organizations create
real-world events. Players join, check in with a secure QR code, and an organizer
verifies participation. Verification awards Eco Points, updates progression, creates a
certificate, records impact statistics, and may asynchronously issue a blockchain
achievement.

## Layering

```
crates/domain          pure rules, no framework types
crates/application     use cases + ports (traits)
crates/infrastructure  adapters: PostgreSQL today, blockchain/queue later
crates/api             Axum HTTP transport, composition root
crates/cli             clap + reqwest operator tool
crates/game-wasm       deterministic presentation-side progression (WebAssembly)
web/                   React + TypeScript + Vite client
blockchain/            placeholder; no program written yet
migrations/            SQLx migrations, applied on API start
```

Dependency direction is strictly inward: `api -> application -> domain`, with
`infrastructure` implementing `application` ports. `domain` depends on nothing but
`serde`/`thiserror`.

## Invariants (enforced from Phase 1 onward)

1. Domain rules stay independent of Axum, SQLx, blockchain, and the frontend.
2. The database is the source of truth for participation and points.
3. Points move through a point-transactions ledger; `users.eco_points` is a derived
   cache, never the sole record.
4. A participation may be verified exactly once.
5. Verification, point awarding, certificate creation, and impact recording are
   idempotent.
6. Blockchain minting is asynchronous and never blocks verification.
7. No private user information is written on-chain.
8. QR tokens are stored hashed, expire, and cannot be claimed twice per event.
9. The Wasm game module computes presentation and unlock state only; it never
   authorizes or awards points.
10. Every endpoint has authorization, validation, clear errors, and tests.

## Error format

All API errors share one envelope so clients branch on `code`, not prose:

```json
{ "error": { "code": "not_found", "message": "event" } }
```

Mapping: `DomainError::Validation -> 400`, `NotFound -> 404`, `Conflict -> 409`,
`Forbidden -> 403`, `AppError::Infrastructure -> 503` (detail logged, never returned).

## Health

`GET /api/health` returns `200` with `{"status":"ok","dependencies":{"database":"up"}}`
when PostgreSQL answers `SELECT 1`, and `503` with `"degraded"`/`"down"` otherwise, so
it doubles as an orchestrator readiness probe.

## Observability

`tracing` + `tracing-subscriber`, filtered by `RUST_LOG`. Set `LOG_JSON=true` for JSON
logs in deployed environments. HTTP spans come from `tower_http::trace::TraceLayer`.

## Configuration

All configuration is environment driven (see `.env.example`). `DATABASE_URL` is the only
required variable; everything else has a development default. The API applies
migrations at startup and exits if PostgreSQL is unreachable; once running, transient
database outages surface as `503` on `/api/health` rather than crashing the process.

## Phase 0 scope

Scaffolding only: workspace, layering, Docker Compose PostgreSQL, config, logging,
error envelope, CORS, health endpoint, migration pipeline, CI, docs.

## Phase 1: identity

Schema (`migrations/20260812010000_auth.sql`): `users`, `refresh_tokens`,
`organizations`, `organization_members`, `audit_logs`, plus the `user_role`,
`user_status` and `verification_status` enums.

Endpoints:

| Method | Path                 | Auth      | Purpose                          |
| ------ | -------------------- | --------- | -------------------------------- |
| POST   | `/api/auth/register` | none      | Create account, open session     |
| POST   | `/api/auth/login`    | none      | Open session                     |
| POST   | `/api/auth/refresh`  | cookie    | Rotate refresh token             |
| POST   | `/api/auth/logout`   | cookie    | Revoke session, clear cookies    |
| GET    | `/api/auth/me`       | access    | Current user profile             |

Decisions:

- **Passwords**: Argon2id, 19 MiB / 2 iterations / 1 lane (OWASP baseline). Only
  the PHC string is stored. Unknown emails still pay a hash so login timing does
  not leak account existence.
- **Access token**: HS256 JWT, 8 hour default (`ACCESS_TOKEN_TTL_SECS`), delivered in the HttpOnly
  `eq_access` cookie (`SameSite=Strict`, `Secure` unless `COOKIES_SECURE=false`).
  Never in `localStorage`; a `Bearer` header is accepted only as a fallback for
  non-browser clients such as the CLI.
- **Refresh token**: 256-bit opaque value in the HttpOnly `eq_refresh` cookie,
  scoped to `Path=/api/auth`. Stored as a SHA-256 hash. Every refresh rotates it;
  replaying a rotated token revokes the user's whole token family.
- **Authorization**: `AuthUser`, `AdminUser` and `OrganizationUser` extractors.
  `Role::satisfies` grants `ADMIN` every permission; other roles must match.
  `ADMIN` is never self-assignable at registration.
- **Organizer authorization**: organization capability is ownership, not a role,
  so the same account gains it once its organization is approved — no second
  login. `EventService` has two guards. `require_owner` covers reads and
  cancellation, which a suspended organization keeps so it can see its data and
  wind events down. `require_approved_owner` covers everything that advances an
  event or mints value — create, edit, publish, activate, issue a check-in code,
  and verify or reject participation — so a pending, rejected, or suspended
  organization cannot award points or certificates.
- **Rate limiting**: fixed window per client IP on all auth endpoints
  (`AUTH_RATE_LIMIT_MAX` per `AUTH_RATE_LIMIT_WINDOW_SECS`), in-process for now.
- **Audit**: `user.registered`, `user.login`, `user.login_failed`,
  `user.login_blocked`, `session.refreshed`, `session.logout`,
  `session.reuse_detected`. Audit failures are logged, never fail the request.
- **Seeding**: `cargo run -p ecoquest-cli -- seed` creates one administrator plus
  test accounts, then loads the full demo dataset from `scripts/seed-full.sql`
  (organizations, events in every lifecycle state, verified participations,
  point ledger, certificates, and an `OCEAN_GUARDIAN` achievement) in one
  transaction. Re-running never overwrites existing accounts or data. A clean
  environment is `make reset`: drops the public schema, applies migrations with
  `cargo run -p ecoquest-cli -- migrate`, then re-seeds.

## Events, check-in, and verification

Schema: `events`, `event_impacts`, `event_participations`, `event_qr_tokens`,
`participation_verifications`, `point_transactions`, `impact_contributions`.

An event moves `DRAFT → PUBLISHED → ACTIVE → COMPLETED`, or `CANCELLED` from any earlier
state; only a published or active event
accepts joins, and capacity is checked inside the join transaction. QR tokens are stored
as SHA-256 hashes, expire, and are rotatable; `UNIQUE (event_id, user_id)` means one
check-in per player per event.

Verification is a single transaction that writes the verification record, flips the
participation to `VERIFIED`, inserts a point transaction, recomputes `users.eco_points`
as the sum of that ledger, records impact contributions, advances achievement progress,
and enqueues one `certificate.issue` outbox row. Recomputing rather than incrementing
means a replayed request cannot inflate a balance.

Two database constraints carry the money rule: points are zero unless the status is
`VERIFIED`, and a rejected participation can never hold points.

## Certificates

A worker in the API process claims outbox jobs with `FOR UPDATE SKIP LOCKED`, so several
instances can run without coordination. It reserves the certificate number and issue
timestamp, then hashes the length-prefixed concatenation of number, user, event,
organization, issue time, and verified minutes with SHA-256. Length prefixes keep a value
from shifting across a field boundary and colliding. The hash is the tamper check: alter
any field and it no longer matches.

Issuance is idempotent on `participation_id`, and a job whose participation is no longer
`VERIFIED` is retired instead of issued.

Reads are authorized in SQL: the participant or a member of the hosting organization may
fetch by participation, and anyone may fetch by verification hash. A caller with no
relationship receives `404`, not `403`, so the endpoint does not confirm existence.

## Wallet linking

The server issues a single-use nonce valid for five minutes. The wallet signs a fixed
message containing the address and nonce, and the server verifies the Ed25519 signature.
Address encoding selects the chain: 56 characters beginning with `G` are decoded as
Stellar StrKey (base32, version byte `0x30`, CRC16-XModem checksum, rejecting muxed `M`
and seed `S` prefixes), anything else as Solana base58. Both chains use Ed25519 keys, so a
single verifier serves both.

This proves key ownership only. `MockBlockchainAdapter` still stands in for minting and
nothing is broadcast to any network.

## Administration

`AdminService` sits behind the `AdminUser` extractor, so every route under `/api/admin`
requires the `ADMIN` role. Migration `20260815010000_admin_moderation.sql` adds soft state
only: no administrator action deletes a row.

- **Reasons**: every mutation requires a non-empty reason. The reason is persisted on the
  target (`organizations.review_reason`, `events.moderation_reason`, `users.status_reason`,
  `participations.flag_reason`) and copied into the audit entry.
- **Audit**: the action names the outcome, not the endpoint —
  `organization.approved` / `.rejected` / `.suspended` / `.reopened` / `.inactivated`,
  `user.role_changed`, `user.suspended` / `.deactivated` / `.reactivated`,
  `user.points_corrected`, `user.password_reset_issued`,
  `event.cancelled` / `.suspended` / `.restored` / `.archived`,
  `participation.flagged` / `.unflagged`. Each records the acting administrator, the
  reason, and the previous value. As elsewhere, an audit write failure is logged and
  never fails the request.
- **Self-protection**: an administrator cannot change their own role or account status.
  A role change revokes the target's refresh-token family, so they cannot start a new
  session. Their existing access token stays valid until it expires, because a JWT
  carries its own claims and is never checked against the database — so a demotion or
  suspension is fully effective only after `ACCESS_TOKEN_TTL_SECS` (8 hours by default).
  Shorten that variable if immediate revocation matters more than long sessions.
- **Event moderation**: cancel, suspend, restore, archive. Each transition is guarded —
  only a published or active event can be suspended, only a suspended one restored.
  `events.previous_status` remembers the state a suspension interrupted so a restore
  returns to it rather than guessing. Moving to cancelled, suspended, or archived revokes
  the event's QR tokens, so a moderated event stops accepting check-ins immediately.
- **Points**: corrections are ledger entries in `point_transactions`, never direct writes
  to the `users.eco_points` cache, and are rejected if they would drive a balance below
  zero.
- **Listing**: `sort` is mapped through a per-resource allowlist to a `&'static str`
  ORDER BY fragment, so no caller string reaches SQL. `per_page` is clamped to 100.
- **User-facing links**: built from `WEB_BASE_URL`, falling back to the first
  `CORS_ALLOWED_ORIGINS` entry. It must be set to the deployed client origin in production.

## Not built

No blockchain program exists and no target network is chosen. Certificates render as JSON,
not PDF. Revocation columns exist in the schema without an endpoint. Password-reset tokens
are minted and stored but cannot be redeemed: there is no redemption endpoint and no
`/reset-password` route in the web client.
