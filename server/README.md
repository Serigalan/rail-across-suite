# rail-across-server

The project server of Rail-Across-Suite — one for the suite and every module
in it: the one sign-in (users and sessions, cookie `ra_session`), projects
with variants and revisions (core 1.0 documents), images, point clouds. It
began as the Layout-Tool's (its roadmap phase 10). Editing and
calculating stay in the browser; the server stores, versions, checks and
manages users. It never merges — it takes a revision only on top of the
current head of a variant (decision 97) and checks it with the browser's own
code (`parseProjectsPayload`, `validateProject`, decision 91).

## Run

Node 22. `npm install` at the repository root (one npm workspace with the apps and packages), then

    node bin/rail-across-server.mjs create-admin <login> [name]   # start password printed, changed at first sign-in
    node bin/rail-across-server.mjs serve
    node bin/rail-across-server.mjs backup <file>                 # consistent copy (SQLite backup API)

The server runs on plain Node, without a build step or a module hook: it checks records with the core
(`@rail-across/core`) and prepares point clouds and runs its long checks with `@rail-across/pointcloud` — the
same code the browser runs. It imports no app.

| Variable | Default | |
|---|---|---|
| `RA_SERVER_DB` | `rail-across.sqlite` | database file (SQLite, WAL) |
| `RA_SERVER_HOST` | `127.0.0.1` | bind address |
| `RA_SERVER_PORT` | `8787` | port |
| `RA_SERVER_TRUST_PROXY` | `127.0.0.1,::1` | addresses/CIDR ranges whose `X-Forwarded-For` is believed (the reverse proxy) |
| `RA_SERVER_INSECURE_COOKIE` | — | `1` drops `Secure` from the cookie (plain-http local runs only) |
| `RA_ADMIN_PASSWORD` | — | start password for `create-admin` instead of a generated one |

The schema lives in `migrations/`, one numbered SQL file per step, applied in
order on start.

## Tests

From `apps/layout-tool/`, with the app's: `npm test` (vitest runs
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
([`deploy/README.md`](../deploy/README.md)).
