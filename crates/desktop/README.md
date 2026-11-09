# rail-across-desktop

Rail-Across-Suite on the desktop (decision 0020, step 9): the whole suite in one Tauri window, the project
server (`crates/server`) in the same process. The window loads the built suite from the URI scheme `rail`
(`rail://localhost/`; `http://rail.localhost/` on Windows); every request on it — pages and the API under
`/api` — is answered by the server's router in this process. No port, no child process, no Node.

The server runs in its local mode: no user management, one implicit local user (`local`), the cloud jobs as
threads. The working files (the database `rail-across.sqlite`, the point clouds under `clouds/`) lie in the
folder chosen on the start screen, which also asks for the language and the export folder — downloads land
there (the local package of the branch `desktop-standalone`). The choices are kept in `settings.json` in the
app data directory; the start screen comes only while there is none.

## Build

Linux needs WebKitGTK and GTK (`libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev librsvg2-dev`). It is a
cargo workspace of its own for that reason; it uses the server and the core's crates by path.

    npm ci                               # at the repository root
    cd crates/desktop                    # everything below in this directory
    node scripts/build-site.mjs          # the suite into site/ (start page, modules, Rust modules)
    node scripts/stage-mdbtools.mjs      # mdb-export and its libraries into mdbtools/, where installed
    cargo run --release                  # the app, on site/
    cargo tauri build                    # the package (with the Tauri CLI): runs the two scripts itself, bundles site/ and mdbtools/

The `package.json` here holds no dependencies: the Tauri CLI takes the directory of the nearest `package.json`
as the frontend's and runs `beforeBuildCommand` there — without it, in `crates/`.

| Variable | |
|---|---|
| `RA_DESKTOP_SITE` | the built site (default: the bundled one, else `site/`, else the repository's `dist/`) |
| `RA_DESKTOP_DATA` | the working files, overriding the start screen's choice |

`--smoke` starts on the defaults, loads the suite and asks the API through the window — `/api/me`,
`/api/health`, a new project, the project list — prints `RA_DESKTOP_SMOKE={…}` and quits, 0 when the
answers are right. CI runs it under Xvfb.
