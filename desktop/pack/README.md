# Rail-Across-Suite — local package

The whole suite on your own computer, completely local: the start page, every module
(Layout-Tool, Signalling-Tool, Fahrzeug-Editor, Fahrplan-Editor) and the project server
with projects, variants, revisions, point clouds and timetables. **No user management** —
no sign-in, no users, no passwords; one implicit local user stands behind every change.

Only `127.0.0.1` is bound: nothing is reachable from other machines. Apart from the map
tiles, terrain heights and basemaps, which the browser loads from their providers as in the
online version, nothing leaves the computer.

## Start

A Node.js runtime (22+) is bundled in `node/bin/` — nothing needs to be
installed. (Only if that folder is missing: install
[Node.js](https://nodejs.org) 22 or later.)

- **Windows:** double-click `start.cmd`
- **Linux / macOS:** run `./start.sh` in a terminal

The server starts on a free port and the browser opens on its address
(`http://127.0.0.1:<port>/`). Stop it with Ctrl+C in the terminal (or close the `start.cmd`
window). The log is in `data/server.log`.

## Where the data lives

`data/rail-across.sqlite` (the database) and `data/clouds/` (the point cloud files) beside
the start script — move the whole folder and the data moves with it. A consistent copy of
the database:

    node server/bin/rail-across-server.mjs backup data/backup.sqlite

(with `RA_SERVER_DB=data/rail-across.sqlite` in the environment.)

## What is inside

- `app/` — the built suite: the start page at the root, each module's app under its path
  (`/layout/`, `/signalling/`, `/vehicles/`, `/timetable/`, …)
- `server/` — the project server with its dependencies installed (`node_modules/`), ready to
  run: `node server/bin/rail-across-server.mjs serve --local --static app --port 8787`
  serves the same thing on a fixed port
