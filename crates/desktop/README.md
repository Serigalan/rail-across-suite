# rail-across-desktop

Rail-Across-Suite on the desktop (decision 0020, step 9): the whole suite in one Tauri window, the project
server (`crates/server`) in the same process. The window loads the built suite from the URI scheme `rail`
(`rail://localhost/`; `http://rail.localhost/` on Windows); every request on it — pages and the API under
`/api` — is answered by the server's router in this process. No port, no child process, no Node.

The server runs in its local mode: no user management, one implicit local user (`local`), the cloud jobs as
threads. The working files (the database `rail-across.sqlite`, the point clouds under `clouds/`) lie in the
folder chosen on the start screen, which also asks for the language and the export folder — downloads land
there (the local package of the branch `desktop-standalone`). The choices are kept in `settings.json` in the
app data directory; the start screen comes by itself only while there is none. The gear at the top right of the
suite's start page opens it again; a change restarts the app (the server keeps its working files for its
lifetime). A language chosen in the suite is kept for the next start as well.

## Portable (Windows)

The Windows package is a ZIP of one folder: `Rail-Across-Suite.exe`, the suite (`site/`), `mdb-export` with its
DLLs (`mdbtools/`) and the two files of [`portable/`](portable/) — the marker `portable` and a readme for the
user (`LIESMICH.txt`, German and English). With the marker beside the executable everything the app keeps lies
in that folder:

| | |
|---|---|
| `settings.json` | the start screen's choices; folders inside the portable folder kept relative to it, so it can move (a USB stick under another drive letter) |
| `data/` | the working files unless chosen otherwise: the database, the point clouds, `rail-across.log` |
| `export/` | exported files unless chosen otherwise |
| `webview/` | WebView2's own storage — the working copies (IndexedDB), the language |

Started from Explorer, a release build has no console (`windows_subsystem = "windows"`): the server's messages go
to `data/rail-across.log`, begun anew past 5 MB. `mdb-export` runs without a console window of its own.
WebView2 is not bundled: it is part of Windows 11 and of current Windows 10; the readme says where to get it
otherwise. The executable is not signed, so SmartScreen warns on its first start.

The release workflow (`.github/workflows/release-desktop.yml`) builds the packages for Windows — the ZIP and the
installers — and macOS — a `.dmg` for Apple silicon —, on a tag `desktop-YYYY-MM-DD[.n]` as a release of that
tag, by hand as the run's artifacts only (the way to try it before the tag). Neither is signed: SmartScreen warns,
macOS refuses the first start until it is allowed (the release notes say how). `mdb-export` comes with the libraries it needs:
from Homebrew on macOS; on Windows, where MSYS2 has no package of it, built from mdbtools' source with MSYS2's
msys gcc, as mdbtools' own CI does there. Linux is not
released; CI builds the app on Linux and Windows and runs the smoke check on both, portable.

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
| `RA_DESKTOP_PORTABLE` | a folder to run portable in, as if the marker lay beside the executable (any platform) |

`--smoke` starts on the defaults, loads the suite and asks the API through the window — `/api/me`,
`/api/health`, a new project, the project list — prints `RA_DESKTOP_SMOKE={…}` and quits, 0 when the
answers are right. CI runs it portable, under Xvfb on Linux and on Windows.

## Logo and icons

`icons/logo.svg` is the logo — the rail profile and the name from `architecture.svg`, on white in a blue frame
with round corners; the start screen shows it. `icons/icon.svg` is the same in a square, `icons/icon-small.svg`
the profile alone for 16–64 px. `node scripts/icons.mjs` makes the PNGs, `icon.ico` and `icon.icns` from them
(headless Chrome).
