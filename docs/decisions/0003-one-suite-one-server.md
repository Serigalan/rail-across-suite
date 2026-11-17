# 0003 — One suite on one server, signing in first, modules named by their task, one build

Status: **accepted** · as of 2026-11-17

## Decision

1. **Rail-Across-Suite is one application on one server.** Its start page, every module's pages (each under its
   own path: the Layout-Tool under `/layout/`) and the project server run behind one address. The project server
   is one Rust program (`crates/server`, 0005) behind Caddy; the deployment is the suite's (`deploy/`).
2. **Signing in comes before the suite.** Nothing of it opens without a session: the server asks the project
   server for every page and shows the sign-in page in its place until one is signed in. A start password is
   changed there before anything opens. One account for every module. Open without a session are only the
   sign-in page, the built files it and the pages load, and the entry of the Layout-Tool's read-only share links,
   whose holders have no account.
3. **The suite manages what is the suite's:** the account (password, signing out) and the users with their roles
   and the modules' rights (`apps/home`, `#/admin`). A module has no sign-in or user administration of its own.
4. **Modules are named by their task**, each an app of its own (`apps/<name>`), on the start page as a tile:

   | Module | Path | What it does |
   |---|---|---|
   | Layout-Tool | `/layout/` | the alignment (0008) |
   | Civil-Tool | `/civil/` | the civil layer; the Layout-Tool's app opened as the Civil-Tool |
   | Signalling-Tool | `/signalling/` | the signalling layer (0009) |
   | Catenary-Tool | `/catenary/` | the electrification layer |
   | Fahrzeug-Editor | `/vehicles/` | the vehicles (0012) |
   | Fahrplan-Sim | `/timetable/`, `/simulation/` | the Fahrplan-Editor and the Simulation-Tool, one tile (0012) |
   | Kapazitäts-Sim | `/capacity/` | capacity after UIC 406 (0013) |
   | Baubetriebs-Sim | `/construction/` | the net day by day, construction scenarios (0014) |

5. **There is one build: the one with the project server.** It runs behind Caddy on the server, or with the
   server in the process in the **desktop app** (Tauri, `crates/desktop`): the window loads the suite from its own
   URI scheme, answered by the server's router in its local mode, without a port and without Node. Working without
   a network is the desktop app's; there is no build that runs in the browser alone.
6. **Work done in one go runs in the server's crate**, native on the server and native in the desktop app: the
   MDB import, the plan's PDF, the point cloud jobs. What answers while someone edits runs in the core's module in
   the page (0005).

## Decided, not built yet

- **The plan carries the suite's mark.** The title block and the logo of the Layout-Tool's plan still show the
  wordmark of the Open Layout Tool (`planTitleBlock.js`, `planLogo.js`, `logoMark.js`). They show the
  Rail-Across mark, with "Layout-Tool" as text.

## Why

- **One account, one place.** Users who plan track layout, signalling and timetables in the same project sign in
  once and are managed once.
- **Nothing to see before signing in.** A start page that showed itself and asked further in told every visitor
  what the suite holds.
- **A module is a part, not a product.** The names say what each part does inside the suite.
- **One build.** Two builds kept every part of the app split in two and a second store beside the working copies;
  the desktop app does what the local build did, with the server's checks and jobs.

## What it costs

- The server's site configuration asks the project server for every page that is not open — one small request
  each, answered from the session table.
- Data kept in browsers keeps its internal names (`olt-…` storage keys), so nothing kept is lost.
- No use of the suite without either a server or the desktop app installed.
