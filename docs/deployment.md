# Deployment

Free-tier topology: **Vercel** serves the client and proxies the API, **Google
Cloud Run** runs the Rust server, **Neon** hosts PostgreSQL.

```
browser ──► https://<project>.vercel.app       (client, static)
                 │  /api/*  proxied server-side, same origin to the browser
                 ▼
            https://ecoquest-api-….run.app     (Cloud Run, scales to zero)
                 │
                 ▼
            Neon PostgreSQL                    (managed, TLS)
```

## Why the proxy is mandatory

Session tokens live in `HttpOnly` cookies set with `SameSite=Strict`
([`crates/api/src/auth/routes.rs`](../crates/api/src/auth/routes.rs)). A browser
will not send a `SameSite=Strict` cookie to a *different site*, so hosting the
client and the API on unrelated domains breaks authentication: login appears to
succeed and every subsequent request answers 401.

Vercel's `/api/*` rewrite makes the API reachable at the project's own origin,
so the cookies stay first-party — the same arrangement the Vite dev server uses
locally. Consequences worth knowing:

- The client sets **no** `VITE_API_URL`. It calls relative `/api/...` paths.
- The API needs **no** CORS entry for the browser, because nothing is cross-origin.
- Relaxing to `SameSite=None` instead would make these third-party cookies,
  which Safari blocks and Chrome restricts. Avoid it.

## 1. Database (Neon)

Create a project at neon.tech and copy the **pooled** connection string. Cloud
Run scales to zero and reconnects often, so connections should terminate at
Neon's pooler rather than directly at the database.

Append TLS enforcement if it is not already present:

```
postgres://USER:PASSWORD@ep-….neon.tech/ecoquest?sslmode=require
```

No migration step is needed. The API applies pending migrations on start-up
([`crates/api/src/main.rs`](../crates/api/src/main.rs)), so the first deploy
builds the schema. To seed demo data afterwards, run the CLI that ships in the
same image:

```bash
docker run --rm -e DATABASE_URL="…" -e SEED_ADMIN_EMAIL="…" -e SEED_ADMIN_USERNAME="…" -e SEED_PASSWORD="…" ecoquest-api seed
```

## 2. API (Cloud Run)

Generate a real signing secret first — the value in `.env.example` is a
placeholder and the API rejects anything under 32 characters:

```bash
openssl rand -base64 48
```

Build and deploy from the repository root:

```bash
gcloud run deploy ecoquest-api --source . --region asia-southeast1 --allow-unauthenticated
```

`--allow-unauthenticated` refers to Cloud Run's own IAM layer, not the
application: the service must be publicly reachable so Vercel can proxy to it.
The app's own authentication is unaffected.

Then set the environment. Keep `JWT_SECRET` and `DATABASE_URL` in Secret
Manager rather than plain variables:

| Variable | Value |
|---|---|
| `DATABASE_URL` | the Neon pooled URL |
| `JWT_SECRET` | the generated secret, ≥32 characters |
| `COOKIES_SECURE` | `true` |
| `WEB_BASE_URL` | `https://<project>.vercel.app` |
| `CORS_ALLOWED_ORIGINS` | `https://<project>.vercel.app` |
| `DATABASE_MAX_CONNECTIONS` | `5` |
| `RUST_LOG` | `info` |
| `LOG_JSON` | `true` |

`WEB_BASE_URL` is what password-reset and certificate links point at, so it must
be the public site rather than a developer machine. `CORS_ALLOWED_ORIGINS` is
not needed by the browser under this topology, but setting it keeps direct
non-proxied callers honest.

Do **not** set `API_BIND_ADDR`. Cloud Run assigns the port through `PORT`, and
the config falls back to it ([`crates/api/src/config.rs`](../crates/api/src/config.rs));
an explicit bind address would override that and the service would fail to
start. Keep `DATABASE_MAX_CONNECTIONS` small — every scaled-out instance opens
its own pool against a free-tier database.

## 3. Client (Vercel)

Create the project once from your machine, which writes the ids CI needs:

```bash
npx vercel@latest link
```

That produces `.vercel/project.json` containing `orgId` and `projectId`. The
directory is gitignored — it must never be committed.

Deploys run in GitHub Actions
([`.github/workflows/deploy-web.yml`](../.github/workflows/deploy-web.yml))
rather than Vercel's own build, because `npm run build` compiles
`crates/game-wasm` to WebAssembly first and needs the Rust toolchain plus a
`wasm-bindgen` matching the version pinned in `Cargo.lock`. The workflow
uploads a finished bundle with `vercel deploy --prebuilt`, so Vercel builds
nothing.

Add three repository **secrets**:

- `VERCEL_TOKEN` — a Vercel access token
- `VERCEL_ORG_ID` — `orgId` from `.vercel/project.json`
- `VERCEL_PROJECT_ID` — `projectId` from the same file

And one repository **variable**:

- `API_ORIGIN` — the Cloud Run origin, e.g.
  `https://ecoquest-api-abc123-as.a.run.app`, with no trailing slash and no
  `/api` suffix

`API_ORIGIN` is deliberately a CI variable rather than a checked-in
`vercel.json`: routing is generated at deploy time from a single source, so
there is no placeholder in the repository that can be forgotten. The workflow
fails fast if the variable is unset.

Pushes to `main` touching `web/`, `crates/game-wasm/`, or `Cargo.lock` then
publish automatically; `workflow_dispatch` triggers a manual deploy.

### What the workflow generates

It writes a Vercel Build Output API v3 bundle:

```
.vercel/output/
  config.json      routes: /api proxy, caching, security headers, SPA fallback
  static/          the contents of web/dist
```

Route order is load-bearing. `/api/*` proxies to Cloud Run first; then
`filesystem` serves real files; anything left over falls back to `index.html`,
without which reloading a deep link such as `/app/missions/<id>` would 404.

## Verifying a deploy

```bash
curl -s https://<project>.vercel.app/api/health
```

A `{"status":"ok","dependencies":{"database":"up"}}` through the Vercel origin
proves the proxy, the service, and the database are all wired up. Then sign in
through the UI and reload a deep link such as `/app/missions/<id>`: the first
exercises the cookie path, the second the SPA fallback.

## Free-tier caveats

- **Cloud Run scales to zero.** The first request after idle pays a cold start
  while the process boots and reconnects. Setting a minimum instance of 1
  avoids it and is no longer free.
- **Neon auto-suspends** idle databases; the first query after suspension is
  slow.
- **Vercel's free tier is for non-commercial use.** If EcoQuest ever takes
  payments, that plan no longer covers it.
- Free-tier terms change often. Confirm current limits before relying on them.

## Alternatives

Any container host works for the API, since the image takes only
`DATABASE_URL` and `PORT`: Fly.io and Koyeb keep a small instance warm,
Render's free tier sleeps after ~15 minutes.

For the client, Netlify is an equivalent swap — the same proxy expressed as a
`/api/* … 200!` line in `_redirects` — and Cloudflare Pages needs a Pages
Function, since its `_redirects` cannot rewrite to external hosts.

GitHub Pages is a poor fit. `github.io` is on the Public Suffix List, so every
API host is cross-site, and Pages cannot proxy — which forces the
`SameSite=None` third-party-cookie route this topology exists to avoid.
