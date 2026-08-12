# HTTP API

Base URL: `http://localhost:8080` only for development. Production requires HTTPS. Browser calls use HttpOnly `eq_access` / `eq_refresh` cookies. CLI may use `Authorization: Bearer`.

Every error is `{"error":{"code":"...","message":"..."}}`. Every response includes `X-Request-Id`; include it in support reports. Auth endpoints allow `AUTH_RATE_LIMIT_MAX` attempts per `AUTH_RATE_LIMIT_WINDOW_SECS` per source IP.

## Workflow

| Method | Path | Auth | Body / result |
| --- | --- | --- | --- |
| POST | `/api/auth/register` | no | `username,email,password`; only `PLAYER` is self-assignable |
| POST | `/api/auth/login` | no | `email,password`; session cookies |
| GET | `/api/auth/me` | yes | profile, including `eco_points` |
| GET | `/api/events` | no | published events |
| POST | `/api/organizations/{organization_id}/events` | organization member | event body; `impacts` are per verified participant |
| POST | `/api/events/{id}/publish` | organizer | no body |
| POST | `/api/events/{id}/activate` | organizer | no body |
| POST | `/api/events/{id}/join` | yes | participation |
| POST | `/api/events/{id}/qr` | organizer | `{}`; opaque check-in code and SVG |
| POST | `/api/check-in` | yes | `{"code":"..."}` |
| POST | `/api/events/{id}/participants/verify` | organizer | `{"participation_ids":["uuid"],"reason":"..."}` |
| GET | `/api/impact` | no | verified impact totals |
| GET | `/api/impact/me` | yes | caller totals |
| GET | `/api/achievements/me` | yes | Ocean Guardian eligibility / mint state |
| GET | `/api/organizations/me/status` | yes | caller memberships with `verification_status` |
| GET | `/api/certificates/participations/{participation_id}` | participant or organization member | issued certificate; 404 to anyone else |
| GET | `/api/certificates/public/{verification_hash}` | no | public verification by 64-char lowercase hex hash |

Event create body has `name`, `description`, `activity_type` (`BEACH_CLEANUP`), `location`, ISO-8601 `starts_at` / `ends_at`, `capacity`, `eco_points`, and `impacts: [{metric,unit,expected_value}]`.

## Missing contracts

Certificates are issued asynchronously: verification writes a durable `certificate.issue` outbox event and the API's certificate worker consumes it, so a certificate appears shortly after verification rather than instantly.

No PDF rendering endpoint exists; certificate routes return JSON facts only. Blockchain minting is still the mock adapter (`mock://solana/...`), not a real Solana transaction. Do not claim on-chain proofs or downloadable PDFs.

## Headers

API sets `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`, `Permissions-Policy`, `Cross-Origin-Opener-Policy`, CSP, and HSTS. Terminating proxy must redirect HTTP to HTTPS; HSTS does not replace that redirect.
