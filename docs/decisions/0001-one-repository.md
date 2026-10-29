# 0001 — One repository for the core, all modules and the services

Status: **accepted**

## Decision

Open-Rail-Designer is one repository (a monorepo). The Rail-Data-Core, every module, the project server and the
Python services live in it, each in a directory of its own:

```
open-rail-designer/
  packages/core/        the core in JavaScript: geometry, tracks, switches, location types, remap, merge, validation
  packages/core-py/     the same core for Python (optimizer, simulations)
  schemas/              JSON Schema per domain, shared test vectors
  apps/layout-tool/     the Open-Layout-Tool
  apps/…                further modules (catenary, signalling, timetable, …)
  server/               one project server for all modules
  services/optimizer/   the Python service
  docs/
```

The Open-Layout-Tool moves into `apps/layout-tool/` with its history in a later step (see the roadmap, phase 0).

## Why

The data model makes the modules depend on each other more tightly than separate repositories can hold:

1. **All modules run on the same core version.** The core carries references in every domain, also in those the
   running module does not understand, and a build that cannot carry a domain refuses the edit
   ([data model](../data-model.md), R3, R4). Two modules on two core versions could not both edit one project.
2. **A change to the core is one commit.** A new field in `TrackLocation` changes the core, the migration, every
   module and their tests together, checked by one CI run. Separate repositories would need one change per
   repository and leave weeks in which projects of mixed versions exist.
3. **The server needs every module.** It validates every check-in with the validators of all domains and
   recomputes carried references in domains the author may not write (S-R5).
4. **JavaScript and Python share test vectors** — canonical JSON, hashes, the rule language, the splice. The
   Open-Layout-Tool already does this inside one repository.
5. **The Open-Layout-Tool is built this way inside** — modules with a facade, a checked dependency order
   (`tools/modules.mjs`), layer rules (`tools/boundary.mjs`), two builds from one code base. The same discipline
   moves one level up.

## Rules that come with it

- **Apps import packages, never each other.** What two apps need belongs in a package. Checked in CI, as the
  Open-Layout-Tool checks its module order today.
- **Ownership per directory** through `CODEOWNERS`; a change to `packages/core` or `schemas/` needs a review by
  the core's owners.
- **Each app has its own build and deployment**; CI tests what a change touches, and the core always.
- **Large binary data stays out of the history** (NTv2 grids, kilometrage tiles, map images): Git LFS, release
  assets or a data repository, referenced by version.

## What it costs

- One clone holds everything; mitigated by keeping binary data out and by sparse checkout where needed.
- Modules cannot be released on a core version of their own — which is the point.
- Moving the Open-Layout-Tool changes the paths of its CI and its deployment scripts once.

## Alternatives considered

- **One repository per module, the core as a versioned package.** Rejected for reasons 1–3: version skew between
  modules makes projects uneditable, and every core change becomes a release train across repositories.
- **The Open-Layout-Tool repository becomes the monorepo** (renamed). Keeps its CI and deployment untouched, but
  the shared toolkit would carry the history and identity of one module; the toolkit's own repository is the
  natural home.
