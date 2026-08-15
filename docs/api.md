# HTTP API

Base URL: `http://localhost:8080` only for development. Production requires HTTPS. Browser calls use HttpOnly `eq_access` / `eq_refresh` cookies. CLI may use `Authorization: Bearer`.

Every error is `{"error":{"code":"...","message":"..."}}`. Every response includes `X-Request-Id`; include it in support reports. Auth endpoints allow `AUTH_RATE_LIMIT_MAX` attempts per `AUTH_RATE_LIMIT_WINDOW_SECS` per source IP.

The **Auth** column means: `no` = anonymous, `yes` = any signed-in user, `organizer` = a member of the owning organization, `admin` = `ADMIN` role.

## Identity

| Method | Path | Auth | Body / result |
| --- | --- | --- | --- |
| POST | `/api/auth/register` | no | `username,email,password`; only `PLAYER` is self-assignable |
| POST | `/api/auth/login` | no | `email,password`; sets session cookies |
| POST | `/api/auth/refresh` | refresh cookie | rotates the refresh token and reissues cookies |
| POST | `/api/auth/logout` | yes | revokes the presented refresh token |
| GET | `/api/auth/me` | yes | profile, including `eco_points` |

## Organizations

| Method | Path | Auth | Body / result |
| --- | --- | --- | --- |
| POST | `/api/organizations` | yes | applies to create an organization; starts `PENDING` |
| GET | `/api/organizations/me/status` | yes | caller memberships with `verification_status` |
| GET | `/api/organizations/{organization_id}/events` | organizer | events owned by the organization, any status |
| POST | `/api/organizations/{organization_id}/events` | organizer | event body; `impacts` are per verified participant |

## Events, check-in, and verification

| Method | Path | Auth | Body / result |
| --- | --- | --- | --- |
| GET | `/api/events` | no | discoverable events: `PUBLISHED` and running `ACTIVE` ones that have not ended |
| GET | `/api/events/{id}` | no | single event |
| PUT | `/api/events/{id}` | organizer | updates a draft event |
| POST | `/api/events/{id}/publish` | organizer | no body |
| POST | `/api/events/{id}/activate` | organizer | no body |
| POST | `/api/events/{id}/cancel` | organizer | no body |
| POST | `/api/events/{id}/join` | yes | participation; open while the event is `PUBLISHED` or `ACTIVE` and has not ended |
| GET | `/api/events/{id}/participants` | organizer | roster with check-in and verification state |
| POST | `/api/events/{id}/qr` | organizer | `{}`; opaque check-in code and SVG. The optional `activates_at`/`expires_at` window must overlap the event window, else 400 |
| POST | `/api/check-in` | yes | `{"code":"..."}`; 409 carries the specific reason (mission not started, ended, not joined, already checked in) |
| POST | `/api/events/{id}/participants/verify` | organizer | `{"participation_ids":["uuid"],"reason":"..."}` |
| POST | `/api/events/{id}/participants/reject` | organizer | same body as verify |
| GET | `/api/me/activities` | yes | caller participations across events |

Event create body has `name`, `description`, `activity_type` (`BEACH_CLEANUP`), `location`, ISO-8601 `starts_at` / `ends_at`, `capacity`, `eco_points`, and `impacts: [{metric,unit,expected_value}]`.

`impacts` are validated against the `impact_metrics` catalogue served by
`GET /api/impact/metrics`, and each `expected_value` is what **one verified volunteer**
contributes, never an event total. A 400 is returned when the metric is not in the
catalogue, the unit is not that metric's canonical unit, the value exceeds the metric's
`max_per_participant`, or a metric is listed twice. Impact aggregates group by
`(metric, unit)` and the community goal matches on both, so free-text units would
silently strand a mission's contribution instead of failing.

## Impact, certificates, and achievements

| Method | Path | Auth | Body / result |
| --- | --- | --- | --- |
| GET | `/api/impact` | no | verified impact totals |
| GET | `/api/impact/community-goal` | no | platform goal progress |
| GET | `/api/impact/organizations/{organization_id}` | no | verified totals for one organization |
| GET | `/api/impact/me` | yes | caller totals |
| GET | `/api/impact/metrics` | no | the impact metric vocabulary an event may declare |
| GET | `/api/certificates/me` | yes | caller certificates |
| GET | `/api/certificates/participations/{participation_id}` | participant or organization member | issued certificate; 404 to anyone else |
| GET | `/api/certificates/public/{verification_hash}` | no | public verification by 64-char lowercase hex hash |
| GET | `/api/achievements/me` | yes | every achievement for the caller (see below) |
| POST | `/api/wallet/challenge` | yes | single-use nonce to sign |
| POST | `/api/wallet/verify` | yes | Ed25519 proof; links a Stellar or Solana address |

`/api/achievements/me` returns one flat list of two kinds, distinguished by `kind`:

- `PLATFORM` — every row of `achievement_catalog`, earned or not, with `progress`/`threshold`
  taken from the caller's `achievement_progress` counters (capped at the threshold) and
  `status` of `EARNED` or `IN_PROGRESS`. Returned for all activity types.
- `ONCHAIN` — rows the caller has become eligible for, carrying the mint `status`,
  `verification_reference`, `wallet_address`, `mint_identifier`, `transaction_signature`,
  and an `explorer_url` derived from the chain adapter. `earned` is true from eligibility
  onward; minting only publishes it.

## Administration

Every route below requires the `ADMIN` role; anonymous callers get 401 and non-admins get 403. Each mutation requires a non-empty `reason` and writes an audit entry recording the acting administrator.

List endpoints share one query string: `q`, `status`, `organization_id`, `activity_type`, `location`, `from`, `to`, `flagged`, `sort`, `order` (`asc`/`desc`), `page` (default 1), `per_page` (default 20, maximum 100). They return `{"items":[...],"total":n,"page":n,"per_page":n}`. `sort` is matched against a fixed allowlist per resource; unknown values fall back to the default ordering.

| Method | Path | Body / result |
| --- | --- | --- |
| GET | `/api/admin/organizations` | paged organizations |
| GET | `/api/admin/organizations/{organization_id}` | detail with members, events, certificates, impact |
| PATCH | `/api/admin/organizations/{organization_id}/status` | `{"status":"APPROVED","reason":"..."}` |
| GET | `/api/admin/users` | paged users |
| GET | `/api/admin/users/{user_id}` | detail with activities, certificates, achievements, ledger |
| PATCH | `/api/admin/users/{user_id}/role` | `{"role":"ADMIN","reason":"..."}`; revokes the target's sessions |
| PATCH | `/api/admin/users/{user_id}/status` | `{"status":"SUSPENDED","reason":"..."}` |
| POST | `/api/admin/users/{user_id}/points` | `{"amount":-50,"reason":"..."}`; writes a ledger correction |
| POST | `/api/admin/users/{user_id}/password-reset` | no body; mints a reset token (see caveat below) |
| GET | `/api/admin/events` | paged events |
| GET | `/api/admin/events/{event_id}` | detail with participants, impacts, QR status |
| POST | `/api/admin/events/{event_id}/moderate` | `{"action":"SUSPEND","reason":"..."}` |
| POST | `/api/admin/events/{event_id}/cancel` | `{"reason":"..."}` |
| POST | `/api/admin/participations/{participation_id}/flag` | `{"flagged":true,"reason":"..."}` |

An administrator cannot change their own role or account status; both return 403. A point correction that would drive a balance below zero is rejected. Password-reset links are built from `WEB_BASE_URL` (falling back to the first `CORS_ALLOWED_ORIGINS` entry), so that variable must be set to the deployed client origin in production.

## Missing contracts

Certificates are issued asynchronously: verification writes a durable `certificate.issue` outbox event and the API's certificate worker consumes it, so a certificate appears shortly after verification rather than instantly.

Password reset is **half built**. `POST /api/admin/users/{user_id}/password-reset` stores a
SHA-256 hashed, one-hour token in `password_reset_tokens` and returns a
`/reset-password?token=...` URL, but nothing can redeem it: there is no redemption endpoint
and the web client has no `/reset-password` route. Issued tokens expire unused. Completing
the flow needs an anonymous redeem endpoint plus `AuthStore` support for updating a password.

No PDF rendering endpoint exists; certificate routes return JSON facts only. Blockchain minting is still the mock adapter (`mock-mint-<hash>` identifiers with `mock://chain/tx/...` explorer URLs), not a real Solana transaction. Do not claim on-chain proofs or downloadable PDFs.

## Headers

API sets `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`, `Permissions-Policy`, `Cross-Origin-Opener-Policy`, CSP, and HSTS. Terminating proxy must redirect HTTP to HTTPS; HSTS does not replace that redirect.
