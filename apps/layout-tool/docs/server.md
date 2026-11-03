# Server parts

The main build adds to the browser app everything that needs a server: projects with variants and revisions,
the working copy, the Länder's DGM1 and the MDB conversion, point clouds on the server and their 3D view. The server
is the suite's — one for every module — and so is the account: the server lets nobody in without a session (the
suite's sign-in page answers in place of every page), a session that ends meanwhile loads the page again for the
sign-in, and a password to change, the users and signing out are on the suite's start page, which sends one back
(`src/server/suite.js`). So are the projects: the tool has no start page of its own. The suite's infrastructure
editor makes, imports, exports and manages them for every tool and asks this one by its address what to do with a
variant — work on it, look at it, show its history, compare or merge two, show what the tool keeps in this browser
([apps/README.md](../../README.md)); every way back leads to the projects there. Read-only share links open through `share.html`, which needs no session. Three places hold it:

| Where | What | Runs |
|---|---|---|
| `src/server/` | the browser parts that talk to a server | in the browser, main build only |
| `server/` | the suite's project server `rail-across-server` and the job runner `rail-across-cloudjobs` | Node 22, SQLite |

Operation (setup, deploy, backups, several instances, production releases) is described in
[`deploy/README.md`](../../../deploy/README.md).

## Architecture rule

**Edited and computed in the browser wherever it works without a server; the server stores, versions and checks.**
No computation is held twice: what builds an alignment against the rules — splicing, reconnecting, optimizing,
the alignment fit — is the optimizer module in Rust, run in the browser in both builds ([optimizer](modules/optimizer.md),
decision 0019 of the suite), and the Node server runs the browser's own JavaScript for parsing, validation and
point-cloud jobs. A panel that needs the server says so when it is not
there instead of silently doing nothing.

| In the browser | On the server |
|---|---|
| creating and editing tracks, elements, switches, crossings, heights, platforms, cross sections | MDB conversion, DGM1 terrain and relief tiles (Python) |
| joining two tracks (splice, with a track spacing), reconnecting, optimizing, the alignment fit (the optimizer module), rule checks, plan/OSRD/exchange/Verm.ESN export | |
| merging and resolving conflicts | sign-in, projects, variants, revisions, images, members (Node) |
| point clouds read on this device | checking every revision; preparing uploaded clouds, long runs over them (Node) |

## `src/server` — browser parts of the main build

`src/server` may import from `src/core`, never the other way round. It docks onto core through the extension
points of `core/extensions.js`, filled by `src/server/register.js` — the first import of `main.jsx`, before
anything renders:

| Extension point | Filled with |
|---|---|
| `importSections` | MDB import and MDB import into DB_REF |
| `sidebarFoot` | service status (are the project server and DGM5 there?) |
| `terrainSources` | the Länder's DGM1 via the project server |
| `basemaps` | "DGM1 der Länder": DGM5 with the service's 1 m relief tiles over it from map zoom 15 |
| `cloudProviders`, `useCloudServer`, `useServerRun`, `useCloudRegistration`, `crossSectionTools`, `workspaceAddons` | point clouds on the server, long runs, re-referencing, the 3D window link |
| `saveProjectMeta` | title and description written to the server's project list too |
| `localStoreSections` | the working copies in the storage dialog |

Main pieces:

- `App.jsx` — the session, what the address asks for (`?project=…&variant=…&action=…`) and the way back to the
  suite's pages (`suite.js`); `collab/` — check-in and conflict dialogs, history page. The projects, their
  members and variants, and the users are the suite's (`apps/home`, `#/infrastructure`, `#/admin`).
- `workingCopies.js`, `workingCopyDb.js` — the working copy of a variant in IndexedDB (`olt` › `workingCopies`):
  the record being edited, the revision it rests on and that revision's record.
- `workingCopySync.js`, `useWorkingCopy.js` — opening a variant, local changes (diff against the base), check-in,
  update; the app polls whether the head moved on.
- `variantMerge.js` — merging one variant into another against their youngest common revision.
- `dataServices.js` — what the project server reads for the app: DGM1 heights and relief tiles, the MDB conversion.
- `panels/` — MDB import; `mdb/` — the MDB import job and its worker.
- `cloud3d/` — the 3D window (`#/cloud3d`, its own bundle with three.js), re-referencing, share links.

### Check-in and merge

The server **never merges** and takes a revision only **on top of the current head** of a variant (otherwise
`409`). Checking in on a stale base comes back as stale; the browser merges the head into the working copy against
their common base (`utils/merge`), shows the conflicts, the user decides, the result is adopted and checked in
again — so nothing is stored that no user has seen. Merging variants works the same way, with the source's head as
second parent. The working copy must be checked in before a variant is merged into it.

### The 3D window

A window of its own (`#/cloud3d?…`) with three.js, loaded only there. It shows the project's server clouds in
their known systems, drawn relative to a rounded centre in the plane of most tracks; levels of detail replace a
tile by its finer ones while a voxel covers more than a pixel and the visible children fit a point budget;
colour by height spans the 2nd–98th percentile (a crane would flatten the scale). Points are picked on the GPU
and refined to the nearest original point within 10 cm. Tracks, routes and the cross-section station come from
the main window over a `BroadcastChannel`, including unsaved changes; without a main window from the checked-in
head. A walk along a track (eye 2.5 m over top of rail, W/S/A/D/R/F) can be shared as a link. Re-referencing
(see [pointCloud](modules/pointCloud.md)) is done here.

## `server` — the project server

Fastify on Node 22, SQLite in WAL mode behind a thin access layer (Postgres only if ever several instances or
spatial queries on the server are needed). It runs the browser's own code on plain Node, the packages
`@rail-across/core` (records, checks, merge) and `@rail-across/pointcloud` (clouds) — no build step, no module
hook, no second implementation.

**Data** (`migrations/`, applied in order on start):

| Table | Content |
|---|---|
| `user`, `session` | users (admin or user, deactivated instead of deleted — revisions keep their author; right "edit point clouds"), sessions by token hash |
| `project`, `project_member` | projects (soft-deleted; templates open to all), members besides the creator |
| `variant` | named states with parent variant and head revision |
| `revision` | immutable: parent(s), author, time, message, `schema_version` (3: a core 1.0 document; 2, oltjson-1.1, migrated when read), the whole project as its core 1.0 document gzip-compressed, the id log, the keys of the validation errors it already had, the image hash |
| `blob` | images by SHA-256 |
| `point_cloud`, `cloud_job`, `point_cloud_transform`, `cloud_run`, `share_link` | clouds and their preparation queue, re-referencings with history, long runs, read-only share links |

**API** under `/api/` on the app's own origin (no CORS): login/logout/me/password; projects (list, create,
patch, delete, members); variants (create, rename/archive, head, revisions); revisions (get, check in, remaps,
merge base); blobs (`PUT /blobs` not tied to a project, so a new project brings its picture); clouds (create,
upload raw chunks, complete, retry, index, tiles, collective range requests, transforms); runs; shares; admin
(users, clouds); `GET /api/health` without a session.

**Every check-in is checked** with the browser's own `parseProjectsPayload` and `validateProject`; only errors that
are new against the base revision are refused. Revisions of an older format are answered with
`409 format_outdated`. Measured axes may only change with the cloud right — compared against base and merge parent
with sorted keys, so a merge brings the other side's axes without the right.

**Access.** Only an admin creates users; passwords of at least 12 characters hashed with argon2id; sessions as
httpOnly, SameSite=Strict cookies idle-expiring after 30 days; login attempts braked after five failures. A project
is visible to its creator, its members and the admins; anyone else gets `404`, not `403` — the existence of a
foreign project is not revealed. Requests must be JSON (CSRF protection); raw cloud chunks and an Access file are the exceptions and
need a custom header no foreign page can send without a preflight. Membership for tile requests is cached 60 s.

**Point clouds.** Uploads come in 8 MB chunks, each with its SHA-256 checked before it is written; the whole file is
hashed at the end. `rail-across-cloudjobs` prepares a cloud into its five levels in one pass (one process per job, 768 MB
heap; a job interrupted by a restart is queued again, not failed), deletes the raw file unless asked to keep it,
and runs the long clearance scans and traces in a queue of their own (two at a time, 320 MB heap each). Tiles are
served by `rail-across-server` with range requests and collective requests of up to 64 ranges, membership checked per
request — not by Caddy's file server, whose forward-auth only checks the sign-in, not the project.

## What the project server reads for the Layout-Tool

Computing happens in the browser — the optimizer module in Rust ([optimizer](modules/optimizer.md), decision 0019
of the suite). What a browser cannot read, the project server reads for the app (`server/src/routes/terrain.js`),
behind the session:

| Endpoint | What it does |
|---|---|
| `POST /api/terrain` | DGM1 heights of the Länder that publish fixed-URL tiles (`server/src/terrain/`): each tile fetched once, kept in memory and on disk (`RA_TERRAIN_CACHE`), read bilinearly |
| `GET /api/dem/<z>/<x>/<y>.png` | Terrain-RGB map tiles (zoom 16–17) from the same DGM1, heights to the centimetre; gaps filled from DGM5 (lossless WebP, decoded with libwebp as WebAssembly), then from the nearest pixel; 404 outside these Länder |
| `POST /api/mdb` | Converts an Access file into its Satzarten as JSON with `mdb-export` (mdbtools), the file removed again at once; bytes, not JSON, with the upload header |

## Decisions

- **Login for everything server-side**; the app files and NTv2 grids stay public (the sign-in page needs them),
  `/api` and the km data are behind the session.
- **Node for the project server** so parsing and validation are the browser's code; **SQLite** until it no longer
  suffices.
- **The browser merges, the server only appends on the head** and checks.
- **Members per project; 404 for the unauthorised.**
- **Point clouds: shared per project on the server, chunked uploads with hashes, served with membership checks.**
- **Computing in the browser, in Rust; reading for the browser on the project server** (decision 0019 of the
  suite): one service to run, and the optimizer in the local build too.
- **The DGM1 relief is drawn from the tiles the server already reads for heights**, not from the Länder's own
  hillshade WMS: only a height per pixel gives the same colour scale, legend and hillshade as DGM5, so the two
  meet without a seam. The tiles carry centimetres (MapLibre's `custom` encoding) — the usual 0.1 m would draw
  terraces into flat ground at under a metre a pixel. Below map zoom 15 DGM5 is as fine as the screen, and
  above it a view needs a dozen DGM1 tiles rather than hundreds. A tile drawn while a Land's portal failed is
  sent `no-store`, so the gap is not kept for the week the others are.
