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

## The local package (`serve --local`)

The desktop app (`desktop/`, Tauri) runs the suite completely local on one machine, without user
management: the server then registers no sign-in, no sessions, no password change and no user or admin
routes. Every request carries one implicit local user (`ensureLocalUser` — the row only stands there so
records that name an author keep their reference), and `GET /api/me` answers it with 200, so the modules'
"signed in?" check passes without a cookie. `GET /api/health` answers `{ status: 'ok', local: true }`.
`create-admin` and `user-count` refuse under `RA_SERVER_LOCAL=1`.

    node bin/rail-across-server.mjs serve --local --static <site> --host 127.0.0.1 --port 0

`--static <site>` serves the built suite from one directory (the start page at the root, each module's app
under its path — the layout `deploy/deploy.sh` builds into `RA_SITE_DIR`, and
`desktop/scripts/build-site.mjs` into `desktop/site/`), with Range requests answered for the km line data
and the cloud tiles. Page and API then share one origin, as behind Caddy. In local mode the bound address
is printed to stdout as `RA_DESKTOP_URL=<address>` — `--port 0` picks a free port — which the desktop shell
reads to open its window. Only `127.0.0.1` should be bound: nothing is reachable from other machines.
`RA_SERVER_LOCAL=1` is the same as `--local`; `RA_SERVER_STATIC` is the `--static` default.

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
| `RA_SERVER_LOCAL` | — | `1` is the local mode (`serve --local`): no user management |
| `RA_SERVER_STATIC` | — | directory of the built suite to serve beside the API (`--static`) |

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
