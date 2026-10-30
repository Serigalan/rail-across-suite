# olt-server

The project server of the Open-Layout-Tool (its roadmap phase 10), from which
the one project server of Rail-Across-Suite grows: users and
sessions, projects with variants and revisions, images. Editing and
calculating stay in the browser; the server stores, versions, checks and
manages users. It never merges — it takes a revision only on top of the
current head of a variant (decision 97) and checks it with the browser's own
code (`parseProjectsPayload`, `validateProject`, decision 91).

## Run

Node 22. `npm install` at the repository root (one npm workspace with the apps and packages), then

    node bin/olt-server.mjs create-admin <login> [name]   # start password printed, changed at first sign-in
    node bin/olt-server.mjs serve
    node bin/olt-server.mjs backup <file>                 # consistent copy (SQLite backup API)

The server runs on plain Node, without a build step or a module hook: it checks records with the core
(`@rail-across/core`) and prepares point clouds and runs its long checks with `@rail-across/pointcloud` — the
same code the browser runs. It imports no app.

| Variable | Default | |
|---|---|---|
| `OLT_SERVER_DB` | `olt.sqlite` | database file (SQLite, WAL) |
| `OLT_SERVER_HOST` | `127.0.0.1` | bind address |
| `OLT_SERVER_PORT` | `8787` | port |
| `OLT_SERVER_TRUST_PROXY` | `127.0.0.1,::1` | addresses/CIDR ranges whose `X-Forwarded-For` is believed (the reverse proxy) |
| `OLT_SERVER_INSECURE_COOKIE` | — | `1` drops `Secure` from the cookie (plain-http local runs only) |
| `OLT_ADMIN_PASSWORD` | — | start password for `create-admin` instead of a generated one |

The schema lives in `migrations/`, one numbered SQL file per step, applied in
order on start.

## Tests

From `apps/open-layout-tool/`, with the app's: `npm test` (vitest runs
`server/test/**` through `fastify.inject` against a database in memory).

## Behind Caddy

The API is served under `/api/` on the app's own origin (no CORS). The
optimizer and terrain (`/optimizer/*`) and the km line data (`/data/km*`) sit
behind the sign-in through `forward_auth` against `GET /api/me` — forward_auth
sends the request's own headers, the session cookie with them — so the Python
service itself stays as it is (decision 102). `GET /api/health` answers without
a session, for the deploy script's check.

The Caddy site, the systemd units (service and daily backup timer) and the
scripts that install them are in the app's `deploy/`
([`apps/open-layout-tool/deploy/README.md`](../apps/open-layout-tool/deploy/README.md)).
