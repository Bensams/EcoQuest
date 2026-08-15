# EcoQuest

Gamified environmental-impact platform. Organizations run real-world events, players
check in with a QR code, organizers verify participation, and verified impact turns into
Eco Points, progression, certificates, and (later) a blockchain achievement.

**Status: feature complete except on-chain minting.** Accounts, organizations, events,
QR check-in, verification, Eco Points, impact dashboards, certificates, and wallet
linking all work end to end against PostgreSQL. Blockchain minting is still a mock
adapter — see [Blockchain status](#blockchain-status).

| Area                     | State    | Notes                                                     |
| ------------------------ | -------- | --------------------------------------------------------- |
| Auth and sessions        | Done     | Argon2id, rotating refresh tokens, rate limit, audit log    |
| Organizations            | Done     | Membership, admin approval, `/api/organizations/me/status`  |
| Events                   | Done     | Draft → published → active lifecycle, capacity, join        |
| QR check-in              | Done     | Hashed rotatable tokens, one claim per event                |
| Verification             | Done     | One transaction: points, impact, achievements, certificate  |
| Eco Points               | Done     | Ledger of transactions; `users.eco_points` is a cache       |
| Impact dashboards        | Done     | Verified participation only                                 |
| Certificates             | Done     | Async worker; JSON payload, no PDF render yet               |
| Progression game         | Done     | Rust → WebAssembly, presentation only                       |
| Wallet linking           | Done     | Ed25519 ownership proof, Stellar and Solana addresses       |
| On-chain minting         | **Mock** | No program deployed; no transaction is broadcast            |
| Web organizer console    | Missing  | Publish, QR, and verify are CLI-only                        |
| Web certificate view     | Missing  | API routes exist and are unused by the client               |

## Blockchain status

`blockchain/` holds a README and nothing else. No program or contract is written or
deployed, and the target network is not yet chosen. `MockBlockchainAdapter` returns
deterministic placeholder identifiers such as `mock-mint-<hash>` with `mock://` explorer
URLs.

Wallet linking is real cryptography and is independent of the above: the server issues a
single-use nonce, the wallet signs it, and the server verifies the Ed25519 signature.
Stellar StrKey and Solana base58 addresses are both accepted, since both use Ed25519 keys.
This proves the user controls the key. It does not put anything on a ledger.

Do not describe any part of this project as running on-chain until an adapter replaces
the mock.

## Requirements

- Rust stable (1.80+) with `rustfmt` and `clippy`
- Node.js 20+ and npm
- Docker with Compose v2

## Setup

### Linux / macOS / WSL

```bash
cp .env.example .env
docker compose up -d postgres
cargo run -p ecoquest-api          # http://localhost:8080
```

In a second shell:

```bash
cd web
npm install
npm run dev                        # http://localhost:5173
```

Verify:

```bash
curl -i http://localhost:8080/api/health
cargo run -p ecoquest-cli -- health
```

### Windows (PowerShell, native)

```powershell
Copy-Item .env.example .env
docker compose up -d postgres
cargo run -p ecoquest-api          # http://localhost:8080
```

In a second PowerShell window:

```powershell
cd web
npm install
npm run dev                        # http://localhost:5173
```

Verify:

```powershell
Invoke-RestMethod http://localhost:8080/api/health
cargo run -p ecoquest-cli -- health
```

`make` is not available in plain PowerShell; run the underlying `cargo` / `npm` commands
above, or use WSL for the `make` targets.

## CLI

`ecoquest` is API-only: it never opens a PostgreSQL connection. Set `ECOQUEST_API_URL` or pass `--api-url`; HTTPS is required except `localhost` development URLs. Login stores API session cookies at `%APPDATA%\\EcoQuest\\session.json` on Windows or `$HOME/EcoQuest/session.json` elsewhere. Keep this file private; do not copy it to another API host.

```powershell
cargo run -p ecoquest-cli -- login --email ben@ecoquest.test --password "$env:SEED_PASSWORD"
cargo run -p ecoquest-cli -- me
cargo run -p ecoquest-cli -- organizations status
cargo run -p ecoquest-cli -- events list
cargo run -p ecoquest-cli -- events create --organization-id <org-uuid> --name 'Beach cleanup' --activity-type BEACH_CLEANUP --location 'North beach' --starts-at '2026-08-13T09:00:00Z' --ends-at '2026-08-13T12:00:00Z' --capacity 50 --eco-points 100
cargo run -p ecoquest-cli -- events generate-qr <event-uuid>
cargo run -p ecoquest-cli -- participants <event-uuid>
cargo run -p ecoquest-cli -- verify <event-uuid> <participation-uuid>
cargo run -p ecoquest-cli -- certificates issue-status <participation-uuid>
cargo run -p ecoquest-cli -- stats --json
```

`verify` asks for confirmation because it awards points and can issue a certificate. Use `--yes` only in reviewed automation. All commands accept global `--json` for scripting; default output is readable key/value tables.

## Seed demo data

Development/test only. `make seed` (or `cargo run -p ecoquest-cli -- seed`) creates the
accounts below plus the full demo dataset from `scripts/seed-full.sql` — approved and
pending organizations, COMPLETED/PUBLISHED/DRAFT events, a verified participation, point
and impact ledgers, a certificate, and a Solana achievement. Re-running never overwrites
existing accounts or data. `make reset` wipes everything (drop schema) then migrate + seed.

| Username    | Email                 | Role                                              |
| ----------- | --------------------- | ------------------------------------------------- |
| `eco_admin` | admin@ecoquest.test   | Platform admin (reviews organizations)            |
| `alex`      | alex@ecoquest.test    | Player with a pending organization application    |
| `ben`       | ben@ecoquest.test     | Player, owner of the approved org and its events  |
| `erwin`     | erwin@ecoquest.test   | Player, verified participant on a completed event |

All accounts share one password: `SEED_PASSWORD` from `.env` (this repo's `.env` sets it to
`qwerty123`; the `.env.example` default is `ChangeMe-Local-1234`).

Certificate issuance is asynchronous: verification queues the job and a background worker
writes the certificate, usually within a second. `GET /api/certificates/public/{hash}`
then verifies it without authentication. There is no PDF render — the endpoint returns
JSON. Minting remains mocked; see [Blockchain status](#blockchain-status).

## Checks

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd web && npm run lint && npm run build
```

Or, with `make` (Linux/macOS/WSL): `make ci`.

## Layout

| Path                     | Purpose                                              |
| ------------------------ | ---------------------------------------------------- |
| `crates/domain`          | Pure domain rules, framework free                    |
| `crates/application`     | Use cases and ports                                  |
| `crates/infrastructure`  | PostgreSQL adapter, migrations runner                |
| `crates/api`             | Axum HTTP API, composition root                      |
| `crates/cli`             | Operator CLI (`ecoquest`)                            |
| `crates/game-wasm`       | Deterministic progression, compiled to WebAssembly   |
| `web/`                   | React + TypeScript + Vite client                     |
| `blockchain/`            | Placeholder; no program written yet                  |
| `migrations/`            | SQLx migrations, applied at API startup              |
| `docs/architecture.md`   | Architecture and invariants                          |

## Configuration

Copy `.env.example` to `.env`. Only `DATABASE_URL` is required; the rest default to
development values. The file contains placeholders only — never commit real secrets.

## Troubleshooting

- `/api/health` returns `503` with `"database":"down"` → PostgreSQL is not up yet; check
  `docker compose ps` and `docker compose logs postgres`.
- Port 5432 already in use → set `POSTGRES_PORT` in `.env` and update `DATABASE_URL`.
