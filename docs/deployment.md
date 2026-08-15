# Deployment

Free-tier topology: **Vercel** serves the client and proxies the API, **Render**
runs the Rust server, **Neon** hosts PostgreSQL. None of the three requires a
credit card to sign up.

```
browser ──► https://<project>.vercel.app       (client, static)
                 │  /api/*  proxied server-side, same origin to the browser
                 ▼
            https://ecoquest-api.onrender.com  (Render, sleeps when idle)
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

Create a project at neon.tech and copy the **pooled** connection string. The
API sleeps and reconnects often on a free instance, so connections should
terminate at Neon's pooler rather than directly at the database.

Render offers its own free Postgres, but those instances **expire 30 days after
creation**, which would take the app down a month after the demo. Neon's free
plan does not expire.

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

## 2. API (Render)

Generate a real signing secret first — the value in `.env.example` is a
placeholder and the API rejects anything under 32 characters:

```bash
openssl rand -base64 48
```

In the Render dashboard choose **New → Blueprint** and point it at this
repository. Render reads [`render.yaml`](../render.yaml), which pins the free
instance type, the Docker runtime, and `/api/health` as the health check — a
bad `DATABASE_URL` then fails the release instead of going live broken.

Render prompts for the values marked `sync: false`:

| Variable | Value |
|---|---|
| `DATABASE_URL` | the Neon pooled URL |
| `JWT_SECRET` | the generated secret, ≥32 characters |
| `WEB_BASE_URL` | `https://<project>.vercel.app` |
| `CORS_ALLOWED_ORIGINS` | `https://<project>.vercel.app` |

The two Vercel URLs are unknown until step 3, so put a placeholder in now and
correct them at the end. `WEB_BASE_URL` is what password-reset and certificate
links point at, so it must be the deployed site rather than a developer
machine. `CORS_ALLOWED_ORIGINS` is not needed by the browser under this
topology, but setting it keeps direct non-proxied callers honest.

Do **not** add `API_BIND_ADDR`. Render assigns the port through `PORT`, and the
config falls back to it ([`crates/api/src/config.rs`](../crates/api/src/config.rs));
an explicit bind address would override that and the service would never
answer.

The first build compiles the whole Rust workspace and takes several minutes.
When it finishes, Render shows the service URL, `https://ecoquest-api.onrender.com`
or similar. Confirm it directly before wiring the client to it:

```bash
curl -s https://<service>.onrender.com/api/health
```

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

- `API_ORIGIN` — the Render origin, e.g. `https://ecoquest-api.onrender.com`,
  with no trailing slash and no `/api` suffix

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

Route order is load-bearing. `/api/*` proxies to Render first; then
`filesystem` serves real files; anything left over falls back to `index.html`,
without which reloading a deep link such as `/app/missions/<id>` would 404.

## 4. Close the loop

The two services each need the other's URL, so one of them is necessarily
configured after the fact. Once Vercel has published, go back to Render and
correct `WEB_BASE_URL` and `CORS_ALLOWED_ORIGINS` to the real
`https://<project>.vercel.app`. Saving them restarts the service.

## Verifying a deploy

```bash
curl -s https://<project>.vercel.app/api/health
```

A `{"status":"ok","dependencies":{"database":"up"}}` through the Vercel origin
proves the proxy, the service, and the database are all wired up. Then sign in
through the UI and reload a deep link such as `/app/missions/<id>`: the first
exercises the cookie path, the second the SPA fallback.

## Free-tier caveats

- **Render sleeps a free service after 15 minutes idle**, and waking it takes
  about a minute, during which visitors see a Render loading page. This is the
  sharpest edge of the no-card setup. Before a demo, open the health endpoint
  a couple of minutes early so the service is already awake.
- **Free instance hours are capped at 750 per workspace per month**, enough for
  one always-available service but not several.
- **Neon auto-suspends** idle databases; the first query after suspension is
  slow.
- **Render's free Postgres expires 30 days after creation**, which is why the
  database is on Neon.
- Exceeding the free bandwidth or build-minute allowance suspends free services
  for the rest of the month when no payment method is on file.
- **Vercel's free tier is for non-commercial use.** If EcoQuest ever takes
  payments, that plan no longer covers it.
- Free-tier terms change often. Confirm current limits before relying on them.

## Alternatives

Any container host works for the API, since the image takes only
`DATABASE_URL` and `PORT`. Google Cloud Run and Fly.io both hold a warm
instance and start faster, but each requires a card on file even inside the
free tier. Koyeb is the closest no-card alternative to Render.

For the client, Netlify is an equivalent swap — the same proxy expressed as a
`/api/* … 200!` line in `_redirects` — and Cloudflare Pages needs a Pages
Function, since its `_redirects` cannot rewrite to external hosts.

GitHub Pages is a poor fit. `github.io` is on the Public Suffix List, so every
API host is cross-site, and Pages cannot proxy — which forces the
`SameSite=None` third-party-cookie route this topology exists to avoid.
