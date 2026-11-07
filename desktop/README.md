# The local package (desktop/)

The whole Rail-Across-Suite in one desktop window, completely local: a Tauri
shell starts the project server as a child process on `127.0.0.1` with a free
port (`serve --local`) and opens its window on the address the server prints.

On the initial start (no `settings.json` yet), a start screen (`splash.html`,
built into the site) shows the logo centered and asks below it, step by step:
the suite's language, in which folder the working files (database, point
clouds) are stored, and in which folder exported files are saved. The choices
are kept in `settings.json` in the app data directory; from the next start on
the app goes straight to the main window with them (delete `settings.json` to
be asked again). Defaults: app data directory, system download folder. The
language is set on every page of the main window (`initialization_script` on
`localStorage['rail-across-language']`), downloads are redirected to the
export folder (`WebviewWindowBuilder::on_download`), the server gets the
working-files folder as `RA_SERVER_DB`/`RA_SERVER_CLOUDS`. Closing the last
window quits the app (and stops the server) — also on macOS, where an app
otherwise stays alive without windows (`RunEvent::WindowEvent::Destroyed`).
The server also serves the built suite (`--static desktop/site`), so the start
page, every module and the API under `/api` share one origin — exactly the
layout Caddy serves in production, and no CORS is needed.

**No user management in the local package.** `serve --local` registers no
sign-in, no sessions, no password change, no user or admin routes (`/api/me`
answers 200 with one implicit local user, so the modules' "signed in?" check
passes without a cookie). The local user's row exists only so the records that
name an author — projects, revisions, variants, images — keep their reference.
The start page hides the sign-out, the password page, the user administration
and the project members dialog in this mode.

## Prerequisites

- Node.js 22 or later (the server runs on plain Node). `npm run runtime` /
  `npm run build` stage a standalone binary in `desktop/runtime/node/bin/`
  (scripts/fetch-node.mjs); the packaged app prefers it, so the dmg/zip run
  without Node.js installed. In a checkout, `node` on `PATH` is used — or set
  `RA_DESKTOP_NODE` to its binary.
- Rust and the Tauri v2 prerequisites (on macOS: Xcode Command Line Tools).
- `npm install` at the repository root (one workspace: apps, packages, server).

## Run

    cd desktop
    npm install          # once: the Tauri CLI
    npm run dev          # builds the site (scripts/build-site.mjs), then `tauri dev`

`npm run build` builds the site and then the app (`tauri build`); the bundle
lands in `src-tauri/target/release/bundle/`. For macOS/Windows icons beyond
the committed PNGs run once: `npx tauri icon src-tauri/icons/icon.png`.

The site is built into `desktop/site/` (gitignored): the start page at the
root, each module's app under its path (`/layout/`, `/signalling/`, …) — the
same layout `deploy/deploy.sh` builds into `RA_SITE_DIR`. `scripts/build-site.mjs`
also fetches the Layout-Tool's km line data and tries to build its two Rust
modules to WebAssembly; without them the app still builds, those functions just
say the module is missing (as in `deploy.sh`).

## Where the data lives

Not in the checkout: the app data directory of the OS user
(`app_data_dir()`, e.g. `~/Library/Application Support/dev.railacross.suite/`):

- `rail-across.sqlite` — the database (projects, variants, revisions, point
  cloud records); `node server/bin/rail-across-server.mjs backup <file>` with
  `RA_SERVER_DB` pointing at it writes a consistent copy;
- `clouds/` — the point cloud files.

## Environment

| Variable | Default | |
|---|---|---|
| `RA_DESKTOP_REPO` | the repository this app was built from | the checkout holding `server/` and `desktop/site/` |
| `RA_DESKTOP_NODE` | bundled `runtime/node/bin/node`, else `node` on `PATH` | the Node.js binary that runs the server |
| `RA_DESKTOP_SITE` | `<repo>/desktop/site`, else the bundled `site/` resource | the built site to serve |

The server is started as
`node <repo>/server/bin/rail-across-server.mjs serve --local --static <site> --host 127.0.0.1 --port 0`
with `RA_SERVER_DB` and `RA_SERVER_CLOUDS` in the app data directory; its bound
address comes back on stdout as `RA_DESKTOP_URL=<address>`, which the shell
reads to open the window. The child process is stopped when the window closes
or the app exits. Only `127.0.0.1` is bound: nothing is reachable from other
machines.
