# Roadmap

Rail-Across-Suite is built around one shared data foundation, the **Rail-Data-Core**. Every module reads and
writes it; none keeps a private model of the railway. The roadmap therefore starts with the core and adds
modules only on top of it. The Open-Layout-Tool is the existing module and the source of the core's first
version: its project format oltjson-1.1 already holds the track network.

The phases are ordered by dependency, not by date. A phase is done when its exit criteria hold.

```
Phase 0  Foundation ─▶ Phase 1  Core from OLT ─▶ Phase 2  Generalised core ─┬▶ Phase 3  Signalling / Catenary
                                                                            ├▶ Phase 4  Operations · Timetable · Capacity
                                                                            └▶ Phase 5  Construction-Sim
                                              cross-cutting: storage & server · exchange · rules · tests
```

## Phase 0 — Foundation

Goal: agree on how decisions are made and bring the code into one repository before it is shared.

Done:

- One repository for core, modules and services ([decision 0001](docs/decisions/0001-one-repository.md));
  directory layout in place.
- `CODEOWNERS` (`.github/CODEOWNERS`); its review becomes required for `main` once a second maintainer is named.
- Licences: code under Apache-2.0, documentation under CC BY 4.0 ([decision 0002](docs/decisions/0002-licences.md)).
- [Contribution guide](CONTRIBUTING.md) with sign-off after the DCO; decision records (`docs/decisions/`).
- [Glossary](docs/glossary.md) German ↔ English and [conventions](docs/conventions.md) (units, signs,
  coordinates, stations, precision), taken over from OLT.
- A check that apps import packages and never each other (`npm run check`, run by CI).
- Project named Rail-Across-Suite ([decision 0007](docs/decisions/0007-project-name.md)); the repository renamed
  to `rail-across-suite` on GitHub.
- [Data model](docs/data-model.md) reviewed by its code owner and accepted; the questions left open name the phase
  that decides them.

- The Open-Layout-Tool moved into `apps/layout-tool/` with its history; its server to `server/`, its Python
  service to `services/optimizer/`; CI and deployment scripts on the new paths. The toolkit gets a new server of
  its own; the running server of the Open-Layout-Tool is not migrated.
- Large binary data out of the history, fetched by version and SHA-256
  ([decision 0008](docs/decisions/0008-data-outside-history.md)): the NTv2 grids as release assets; the
  kilometrage tiles not published at all.
- Third-party material listed in [THIRD-PARTY.md](THIRD-PARTY.md): the NTv2 grids with their licences; the
  kilometrage data outside the repository and every published build; the guideline is the project's own
  (CC BY 4.0); the values from the DB guidelines may be taken over.

Open:

- Upload the NTv2 grids as the assets of release `data-1` (until then they come from cdn.proj.org).
- The name ([decision 0007](docs/decisions/0007-project-name.md)): *Across* checked in the registers of the DPMA
  and the EUIPO before the first release.

**Exit:** the data model is reviewed, and every question left open names the phase that decides it; the
Open-Layout-Tool builds, tests and deploys from this repository.

## Phase 1 — Rail-Data-Core v1.0, extracted from Open-Layout-Tool

Goal: the core exists as a package of its own and OLT runs on it **without any change in behaviour** — what it
keeps and exchanges is core 1.0, already in the data model's shape
([decision 0009](docs/decisions/0009-core-1-0-in-the-data-model.md)).

Done:

- The JavaScript parts are one npm workspace; the core is the package `@rail-across/core` in `packages/core`.
- The model parts of OLT moved into the core with their history and tests: geometry, rules and their
  catalogues, track and switch records, ports and remaps, heights, cross sections, topology, commands, measured
  axes, the project record (load/hydrate/dehydrate/validate), diff and merge
  ([packages/core](packages/core/README.md)). The app imports them as `@rail-across/core/<module>`; the project
  server checks records with the core. The same 2649 tests pass, and the main build is the same.
- The three remap defects the data model cases found, fixed
  ([data model §6](docs/data-model.md#6-consequences-for-the-open-layout-tool)): the merge carries all three
  states before it compares, and a platform's location is one value (F2); a reversed or spliced track carries
  its platforms (F3). Both cases are tests and pass.
- Plain Node loads the core without a bundler or a module hook: its imports name their files, JSON comes in by
  attribute, and the catalogue files a bundle carries are read by the app (`tools/modules.mjs` in the core holds it
  to that).
- The point clouds' computation is the package `@rail-across/pointcloud` (readers, tiles, sections, rail
  detection and trace, registration, clearance), and the turnout plane the clearance scan needs is the core's: the
  project server imports no app and runs on plain Node without a module hook.
- **core 1.0** ([schemas](schemas/README.md), [document](packages/core/docs/document.md)): JSON Schemas for
  the domains `core` and `layout` with the core types `TrackEnd`, `TrackLocation`, `TrackRange`, `TrackPath`,
  `Lifespan`, `Ref`; platforms and end marks located by them, named runs of tracks as `trackPaths`, the planning
  status as lifespans over the stage list (existing, final), stations as an operational point register, bulk point
  arrays as blobs; collections sorted by id, canonical JSON after RFC 8785, a manifest of hashes per domain and
  collection; a project as one file, `.railacross.json`, carrying its schemas. The types derived from the schemas
  (`packages/core/types/document.d.ts`).
- The one door: a document is checked whole (envelope, versions, schema hashes, every object against its schema,
  blob hashes) on the way in and on the way out. oltjson-1.1 is migrated at the door — the first step of the
  migration chain —, losslessly both ways (the PEK data set is a test). Ids stay on tracks, not on their elements
  ([decision 0004](docs/decisions/0004-ids-on-tracks.md)). Files and the project server hold core 1.0 documents;
  the server migrates its oltjson-1.1 revisions when it reads them. The app keeps its record as the model it
  works on.
- The Python core `packages/core-py` (no dependencies): the same door, canonical JSON, hashes and manifest, held
  with the JavaScript core to the shared vectors in `schemas/vectors/`.

Open:

- Nothing of phase 1's own. The name check (phase 0) and the release `data-1` are still to do.

**Exit:** OLT imports its model only from the core; all OLT tests and the PEK data set pass; an oltjson-1.1
file opens and saves losslessly; the cases F2 and F3 (reversal and splice) of the data model are tests and
pass. — Reached.

## Phase 2 — Generalised core

Goal: the core can carry what the planned modules need, before they are written.

The format has this phase's shape since core 1.0 (decision 0009): domains, location types, lifespans, the
register, canonical JSON and the manifest exist as data. What this phase adds is the behaviour behind them, and
the app working on the document instead of its record.

- The app works on the core 1.0 document; the record and the conversion go.
- Location types `TrackLocation`, `TrackRange`, `TrackEnd`, `TrackPath`, `Ref` (in the schema since core 1.0) as
  values the core computes with, and the anchor `km` beside station and plane. Before they are written: a look
  at libLRS, the linear referencing library of the OpenRail Association.
- References declared in each domain's schema through these types (`x-target`); the schema travels with the data.
- **One** remap in the core that carries every declared reference in every domain — also in domains the running
  module does not understand; platforms and end marks moved onto it.
- Reverse index (what stands on a track) derived on load; deleting leaves other domains' objects orphaned with a
  finding instead of deleting them.
- **An orphaned object keeps its place in the plane.** When its track goes, the core converts its location into
  plane coordinates (easting, northing, EPSG, and the direction as a bearing) in the same step, because a station
  means nothing without its track. The object keeps its id and every reference to it, so it can be attached to
  a new track later.
- **Construction stages** in the core: the ordered stage list, lifespans `{ from, to }` on every object, the
  planning status derived from them, a stage as a view with every check run per stage, replacements naming
  their predecessor, containment as a check, and two-end connectors (`link`) also within one plane for parts
  of a track that are rebuilt ([decision 0003](docs/decisions/0003-construction-stages.md)). The stage list and
  lifespans are in the format since core 1.0 (oltjson-1.1's status converted at the door: new = from the final
  stage, removal = up to it); the app holds and edits stages of a project's own.
- The operational point register (in the format since core 1.0, drawn from the station texts) edited as such.
- Domains with ownership: `core` (net, platforms, end marks, structures, operational point register, designed
  kilometrage) and discipline domains that store references only into the core.
- Storage as a hash tree on the server and in the browser: revision → manifest → domain → collection, the blobs
  stored once (the manifest, the blobs and canonical JSON exist since core 1.0); the exchange container
  (`.railacross`) with its `mimetype` entry.
- Server: rights per domain; a domain the author may not write may only change by carrying or by orphans
  moved to their plane position.
- **Sketches** ([decision 0005](docs/decisions/0005-sketch-mode.md)): variants with a scope domain that hold only
  the core and that domain; the takeover as a merge limited to the scope, with carrying through the id logs and
  orphans in the plane for the sketch's own tracks; the server's refusal to merge a sketch back; orphans as
  warnings, attached again by their plane position.
- Migration chain per domain at the one door.
- Decided in this phase ([data model §8](docs/data-model.md#8-open-questions)): the tolerance for attaching
  orphans again, and what a change of the designed kilometrage does to objects anchored by km.

**Exit:** a test domain of located objects, unknown to the layout tool, survives split, join, reversal, length
change, deletion, a concurrent merge and a format migration — every object in the right place or orphaned with
a finding and its plane position; JavaScript and Python give the same hashes for the shared vectors; a part of
a track rebuilt over three stages gives a valid network in every stage; a sketch with a crossover of its own is
taken over with its signals carried or orphaned in the plane, and its crossover listed as a proposal.

## Phase 3 — First new infrastructure modules

Goal: prove the core with a second real module.

- **Open-Catenary-Tool**: masts, cantilevers, contact wire height and stagger, tension and feeding sections —
  located objects and ranges along the track, reading geometry, cant and heights from the core.
- **Open-Signalling-Tool**: signals, detection points, balises, track sections, signalling routes, speed
  profiles; station boundaries stored as references to what marks them (entry signals, trapezoid boards, entry
  switches, platforms), with a special-case finding where they differ from EBO §4; export towards PlanPro /
  EULYNX.
  *Done:* the domain `signalling` 1.0 after PlanPro 1.10 with overlaps and flank protection, checked against the
  PlanPro plan P-Hausen; the catalogue of all Ril 301 aspects with the Ks signal kinds and PZB; its content check
  with track sections derived over the net; the core carrying its locations on every track edit and merge (R4,
  R7); reading PlanPro plans; the server checking the domain at check-in; the Signalling-Tool's first version
  ([decision 0011](docs/decisions/0011-signalling-domain.md)) on the suite's look as a library, `@rail-across/ui`
  ([decision 0012](docs/decisions/0012-ui-library.md)). *Next:* the values of Ril 819 and Ril 483, level
  crossings, block and the other parts of a station plan, writing PlanPro.

**Signalling goes first**: it needs locations, ranges, functional objects and topology, and so tests the
generalised core most. Catenary follows.

- **Coordination checks** between the disciplines, derived from shared locations and storing only findings
  (signal on a mast, El signals before a neutral section, signal sight past masts).
- **Adaptations of OSRD** for Open-Capacity-Sim agreed with OSRD's maintainers while the signalling domain takes
  shape — DB signalling systems first ([decision 0006](docs/decisions/0006-capacity-on-osrd.md)).
- Decided in this phase: who writes a coordination domain, should a relation between disciplines not be
  derivable from location ([data model §8](docs/data-model.md#8-open-questions)).

**Exit:** the module works on the same project as OLT; editing tracks in OLT keeps the module's objects placed.

## Phase 4 — Operations, timetable, capacity

- `operations` domain: train types, rolling stock, train runs, stopping patterns.
- **Open-Timetable-Tool**: timetable construction on operational points and paths.
- **Open-Capacity-Sim builds on OSRD** ([decision 0006](docs/decisions/0006-capacity-on-osrd.md)): an adapter
  from one construction stage of a project to OSRD's infrastructure and timetable, and OSRD's results back as
  derived results; OSRD at a pinned version, its adaptations contributed upstream.

## Phase 5 — Construction simulation

Construction stages and lifespans are part of the core from phase 2
([decision 0003](docs/decisions/0003-construction-stages.md)); this phase builds on them.

- `construction` domain: closures, temporary speed restrictions and the tracks in operation per stage.
- The stages' periods and the work between two stages — what ends and begins there — as the base of the
  construction sequence; activities, durations and resources.
- Every stage checked as a network with the same validation, signalling and, later, the capacity simulation.

## Cross-cutting, in every phase

| Topic | Direction |
|---|---|
| Storage and server | one project server for all modules: users, projects, variants and sketches, immutable revisions, merge in the browser, validation on check-in, approvals pinned by hashes and checked by the diff (data model S-R6) |
| Exchange | RailJSON, railML 3, IFC 4.3, PlanPro, LandXML, Verm.ESN — translations at the edge, never the working format |
| Rules | rule and form catalogues as data (as OLT's `constraints/`), shared by JavaScript and Python |
| Tests | chain invariants, shared vectors JS/Python, real data sets, architecture checks for layers and domains |
| Sketch mode | every app opens an infrastructure as a sketch of its own domain; the way back is the takeover ([decision 0005](docs/decisions/0005-sketch-mode.md)) |
| Documentation | one page per core module and domain, with its decisions |

See [docs/data-model.md](docs/data-model.md) for how the data is structured and stored.
