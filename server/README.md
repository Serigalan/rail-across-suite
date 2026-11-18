# rail-across-server

The project server of Rail-Across-Suite — one for the suite and every module
in it: the one sign-in (users and sessions, cookie `ra_session`), projects
with variants and revisions (infra-data-1.0 documents), images, point clouds. It
began as the Layout-Tool's (its roadmap phase 10). Editing and
calculating stay in the browser; the server stores, versions, checks and
manages users. It never merges — it takes a revision only on top of the
current head of a variant (decision 97) and checks it as the browser does
(`parseProjectsPayload`, `validateProject`, decision 91 — in the core's crates).

The server is Rust: `crates/server` (decision 0005) — axum, rusqlite, argon2id; the cloud jobs as
threads; the DGM1 terrain and the MDB import. It is the only one that runs: in the deployment (deploy/), in
the desktop app (`crates/desktop`, in its own process) and in development. This directory holds the schema
(`migrations/`, built into the binary) and the server's tests (`test/`), written for the Node server it
replaced and run against this one since (decision 0005 took the Node server out).

## Run

    cargo build --release -p rail-across-server
    target/release/rail-across-server create-admin <login> [name]   # start password printed, changed at first sign-in
    target/release/rail-across-server serve [--local] [--static <site>] [--host <h>] [--port <n>]
    target/release/rail-across-server backup <file>                 # consistent copy (SQLite backup API)
    target/release/rail-across-server user-count | migrate
    target/release/rail-across-server cloudjobs serve | run <job> | exec <run>

For development, beside the apps' `npm run dev` (they pass `/api` on to `127.0.0.1:8787`):

    RA_SERVER_INSECURE_COOKIE=1 RA_SERVER_DB=/tmp/rail-across.sqlite cargo run --release -p rail-across-server -- serve

## The local package (`serve --local`)

The local mode runs the suite completely local on one machine, without user management — the desktop app
(`crates/desktop`, Tauri) runs it inside its own process through the server's `local` setting, the same mode
`serve --local` gives: the server then answers no sign-in, no sessions, no password change and no user or
admin routes. Every request carries one implicit local user (the row only stands there so records that name
an author keep their reference), and `GET /api/me` answers it with 200, so the modules' "signed in?" check
passes without a cookie. `GET /api/health` answers `{ "status": "ok", "local": true }`. `create-admin` and
`user-count` refuse under `RA_SERVER_LOCAL=1`.

    target/release/rail-across-server serve --local --static <site> --host 127.0.0.1 --port 0

`--static <site>` serves the built suite from one directory (the start page at the root, each module's app
under its path — the layout `deploy/deploy.sh` builds into `RA_SITE_DIR`, and
`crates/desktop/scripts/build-site.mjs` into `crates/desktop/site/`), with Range requests answered for the km
line data and the cloud tiles. Page and API then share one origin, as behind Caddy. In local mode the bound
address is printed to stdout as `RA_DESKTOP_URL=<address>` — `--port 0` picks a free port. Only `127.0.0.1`
should be bound: nothing is reachable from other machines. `RA_SERVER_LOCAL=1` is the same as `--local`;
`RA_SERVER_STATIC` is the `--static` default.

The server checks records with the core's crates (`checkRecord`: `validateProject`, `checkSignalling`) and
prepares point clouds with `crates/pointcloud` — held to the browser's JavaScript by the replay vectors
(`packages/core/replay/`).

| Variable | Default | |
|---|---|---|
| `RA_SERVER_DB` | `rail-across.sqlite` | database file (SQLite, WAL) |
| `RA_SERVER_CLOUDS` | `clouds/` beside the database | point clouds: uploads and tiles |
| `RA_SERVER_OSRD` | `osrd/` beside the database | OSRD's nets for *Fahrzeit rechnen*: RailJSON files (`<name>.json`) laid there by hand, listed under `GET /api/osrd` and handed out under `GET /api/osrd/<name>` to whoever is signed in |
| `RA_SERVER_HOST` | `127.0.0.1` | bind address |
| `RA_SERVER_PORT` | `8787` | port |
| `RA_SERVER_TRUST_PROXY` | `127.0.0.1,::1` | addresses/CIDR ranges whose `X-Forwarded-For` is believed (the reverse proxy) |
| `RA_SERVER_INSECURE_COOKIE` | — | `1` drops `Secure` from the cookie (plain-http local runs only) |
| `RA_SERVER_LOG` | `info` | `info` and above write every request to stdout (time, address, method, path without query, status, duration); `error` only failures; `silent` nothing |
| `RA_SERVER_JOBS` | — | `1` runs the cloud jobs as threads of `serve` (the deployment runs `cloudjobs serve` as a unit of its own) |
| `RA_CLOUDJOBS_PARALLEL` | `2` | point cloud preparations at once |
| `RA_CLOUDRUNS_PARALLEL` | `2` | long runs (clearance, rail tracing) at once |
| `RA_ADMIN_PASSWORD` | — | start password for `create-admin` instead of a generated one |
| `RA_SERVER_LOCAL` | — | `1` is the local mode (`serve --local`): no user management |
| `RA_SERVER_STATIC` | — | directory of the built suite to serve beside the API (`--static`) |
| `RA_TERRAIN_CACHE` | — | where the Länder's DGM1 tiles are kept |
| `RA_MDB_EXPORT`, `RA_MDB_LIBS`, `RA_MDB_TIMEOUT` | `mdb-export` | the Access import's `mdb-export` |

The schema lives in `migrations/`, one numbered SQL file per step, applied in
order on start.
Steps that change the data rather than the schema are in `crates/server/src/data_migrations.rs`: each runs once when `serve` starts,
before it listens, and is recorded in `data_migration` with what it did. The first closes every head of every variant
still worked on where its joints missed by up to a millimetre, as a revision of its own by the inactive user `system`
(decision 0005: the same place is 1 µm).

## Tests

    cargo build --release -p rail-across-server
    cd server && npx vitest run

The tests of `test/` run against the Rust server over HTTP, on a database file of their own, in its test
mode (`RA_SERVER_TEST=1`: the clock from `x-ra-test-now`, the peer's address from `x-ra-test-remote`).
In test mode
the server also answers `/api/__test/*`: users made, the cloud queues worked off in its process, a run claimed,
carried out, queued again or pruned on the test's clock. `test/commands.test.js` holds what is not the API —
the migrations, the data step, the command line with the cloud jobs' `run` and `exec`, the log, and the MDB
import of a real file where one is at hand. The crate's own tests (`cargo test -p rail-across-server`) cover
the terrain and the MDB's CSV; the point clouds' code is `crates/pointcloud`'s, held by its replay.

## Behind Caddy

The API is served under `/api/` on the app's own origin (no CORS). Everything
else of the suite — its pages and the km line data (`/data/km*`) — sits behind
the sign-in through `forward_auth` against `GET /api/me` (decision 102):
forward_auth sends the request's own headers, the session cookie with them.
`GET /api/health` answers without a session, for the deploy script's check.

The Caddy site, the systemd units (service and daily backup timer) and the
scripts that install them are in the app's `deploy/`
([`deploy/README.md`](../deploy/README.md)).
