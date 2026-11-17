# 0001 — One repository, one licence, large data outside the history

Status: **accepted** · as of 2026-11-17

## Decision

1. **One repository** (a monorepo) holds the core, every module, the server, the desktop app and the
   simulations, each in a directory of its own:

   ```
   apps/         the modules' pages, one directory per module, named by its task (0003)
   packages/     the JavaScript the apps share: core (the face of the Rust core), ui, running-times, timetables,
                 vehicles, capacity, working-copy, pointcloud, basemaps
   crates/       the Rust crates: doc, geom, rules, track, switch, commands, signalling, planpro, capacity,
                 pointcloud, survey, project, server, core-wasm, pointcloud-wasm, desktop, …
   sim/          the running time simulation, OSRD's envelope simulation ported to Rust (0011)
   crossover/    the search of the extended switch connection
   optimizer/    the track optimizer and the fits
   server/       the server's database schema and its HTTP tests
   schemas/      a JSON Schema per domain and version, the shared test vectors
   deploy/ docs/ tools/
   ```

   All Rust is one Cargo workspace (0005).

2. **Rules that come with it:**
   - Apps import packages, never each other; what two apps need belongs in a package (`tools/boundaries.mjs`, run
     by CI).
   - One CI run checks a change: the core always, every app and crate in a job of its own.
   - Ownership per directory through `.github/CODEOWNERS`.

3. **Everything is under the GNU Lesser General Public License v3.0 only** (`LGPL-3.0-only`): code, schemas,
   documentation and the project's own pictures alike. `LICENSE` holds the LGPL, `LICENSE-GPL` the GPL-3.0 it
   builds on. OSRD's simulation core, ported to Rust, and OSRD's example vehicles are in the repository under the
   same licence.

4. **Third-party material keeps its own licence** and is named in `THIRD-PARTY.md`. Contributions are signed off
   after the Developer Certificate of Origin 1.1 (`Signed-off-by:`); there is no contributor licence agreement.

5. **Large binary data stays out of the history.** A module that needs such data names each file in a manifest
   (`data.json`) with its size, its SHA-256 and the addresses it can be fetched from; `tools/data.mjs` fetches
   what is missing and accepts a file only with that hash. The first address is a release asset of this
   repository (`data-<n>`).
   - **NTv2 grids:** release `data-1`, and cdn.proj.org, byte for byte the same.
   - **Kilometrage lines:** not published, their source does not allow it. Whoever has them puts them into
     `apps/layout-tool/tiles/`; the script links them into the app only where they are.
   - **Small pictures** (basemap previews, project pictures; a few kB each) stay in the repository.

## Why

- **The modules depend on each other more tightly than separate repositories can hold.** All of them run on one
  version of the format; a change of the core is one commit with every module and its tests; the server checks
  every check-in with the checks of every domain.
- **One licence removes a boundary through the middle of features.** The suite carries a port of OSRD, which is
  LGPL-3.0. Code may move between the two projects in both directions. The LGPL keeps improvements to the suite
  open for whoever passes it on, and still lets its libraries be used by other programs, also closed ones.
- **The history never shrinks.** A file once committed is carried by every clone for good, and data that may not
  be published must not be in a public history, not even as an old blob. A hash makes the version exact.

## What it costs

- One clone holds everything; kept small by keeping binary data out.
- Modules cannot be released on a version of the format of their own — which is the point.
- A vendor who changes the suite and distributes it must publish the changes. Running it as a hosted service
  without passing it on obliges to nothing (unlike AGPL).
- The LGPL is a licence for software; for texts and pictures it is unusual.
- One more step before tests and builds (`tools/data.mjs`); the kilometrage lines are not there for whoever
  clones the repository, and the tests that need them are skipped.

## Alternatives considered

- **One repository per module, the core as a versioned package**: version skew between modules makes projects
  uneditable, and every change of the core becomes a release train across repositories.
- **Apache-2.0 for the suite, the LGPL code apart**: two repositories or two licences in one, for one feature.
- **AGPL-3.0 or GPL-3.0 for everything**: stricter than OSRD, or the suite's packages unusable by programs under
  other licences.
- **Git LFS or a data repository for the large data**: GitHub's quota used up by a few clones, and the data that
  may not be published still in a public store.
