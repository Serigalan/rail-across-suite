# Roadmap

Open-Rail-Designer is built around one shared data foundation, the **Rail-Data-Core**. Every module reads and
writes it; none keeps a private model of the railway. The roadmap therefore starts with the core and adds
modules only on top of it. The Open-Layout-Tool is the existing module and the source of the core's first
version: its project format oltjson-1.1 already holds the track network.

The phases are ordered by dependency, not by date. A phase is done when its exit criteria hold.

```
Phase 0  Foundation ─▶ Phase 1  Core from OLT ─▶ Phase 2  Generalised core ─┬▶ Phase 3  Catenary / Signalling
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

Open:

- **Move the Open-Layout-Tool** into `apps/open-layout-tool/` with its history; its server to `server/`, its Python
  service to `services/optimizer/`; CI and deployment scripts on the new paths. The toolkit gets a new server of
  its own; the running server of the Open-Layout-Tool is not migrated.
- Large binary data (NTv2 grids, kilometrage tiles, map images) out of the history: Git LFS, release assets or a
  data repository, referenced by version.
- Third-party material listed in [THIRD-PARTY.md](THIRD-PARTY.md): the NTv2 grids come with their licences;
  the kilometrage data moves into test fixtures and leaves the published apps; the guideline is the project's
  own (CC BY 4.0); the values from the DB guidelines may be taken over.
- Review of the data model by its code owner (its open questions are answered).

**Exit:** the data model proposal is reviewed and its open questions are answered; the Open-Layout-Tool builds,
tests and deploys from this repository.

## Phase 1 — Rail-Data-Core v1.0, extracted from Open-Layout-Tool

Goal: the core exists as a package of its own and OLT runs on it **without any change in behaviour**.

- Move the model parts of OLT into the core: geometry (planes, elements, transitions, integrator), track and
  switch records, ports and remaps, heights, topology, project load/hydrate/dehydrate/validate, diff and merge.
- JSON Schema for the `core` domain; schema-derived types; the one door (`parse` → `validate`).
- Python package reading the same schema (for optimizer and simulations), held to shared test vectors.
- Conversion oltjson-1.1 → core 1.0 at the door (the first entry of the migration chain).
- The three remap defects the data model cases found, fixed on the way
  ([data model §6](docs/data-model.md#6-consequences-for-the-open-layout-tool)): the merge carries all three
  states before it compares, and a reversed or spliced track carries its platforms.

**Exit:** OLT imports its model only from the core; all OLT tests and the PEK data set pass; an oltjson-1.1
file opens and saves losslessly; the cases F2 and F3 (reversal and splice) of the data model are tests and
pass.

## Phase 2 — Generalised core

Goal: the core can carry what the planned modules need, before they are written.

- Location types `TrackLocation`, `TrackRange`, `TrackEnd`, `TrackPath`, `Ref`, each one value, with the anchors
  station / km / plane.
- References declared in each domain's schema through these types; the schema travels with the data.
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
  of a track that are rebuilt ([decision 0003](docs/decisions/0003-construction-stages.md)). The status of
  oltjson-1.1 converts at the door: new = from the final stage, removal = up to it.
- Domains with ownership: `core` (net, platforms, end marks, structures, operational point register, designed
  kilometrage) and discipline domains that store references only into the core.
- Storage as a hash tree: revision → manifest → domain → collection; bulk point arrays as blobs; canonical JSON
  after RFC 8785, the same bytes from JavaScript and Python; the exchange container.
- Server: rights per domain; a domain the author may not write may only change by carrying or by orphans
  moved to their plane position.
- **Sketches** ([decision 0005](docs/decisions/0005-sketch-mode.md)): variants with a scope domain that hold only
  the core and that domain; the takeover as a merge limited to the scope, with carrying through the id logs and
  orphans in the plane for the sketch's own tracks; the server's refusal to merge a sketch back; orphans as
  warnings, attached again by their plane position.
- Migration chain per domain at the one door.

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
  profiles; export towards PlanPro / EULYNX.

**Signalling goes first**: it needs locations, ranges, functional objects and topology, and so tests the
generalised core most. Catenary follows.

- **Coordination checks** between the disciplines, derived from shared locations and storing only findings
  (signal on a mast, El signals before a neutral section, signal sight past masts).

**Exit:** the module works on the same project as OLT; editing tracks in OLT keeps the module's objects placed.

## Phase 4 — Operations, timetable, capacity

- `operations` domain: train types, rolling stock, train runs, stopping patterns.
- **Open-Timetable-Tool**: timetable construction on operational points and paths.
- **Open-Capacity-Sim**: running times and capacity from geometry, speed profiles, signalling and timetable
  (export to OSRD RailJSON as a first back end).

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
| Storage and server | one project server for all modules: users, projects, variants, immutable revisions, merge in the browser, validation on check-in |
| Exchange | RailJSON, railML 3, IFC 4.3, PlanPro, LandXML, Verm.ESN — translations at the edge, never the working format |
| Rules | rule and form catalogues as data (as OLT's `constraints/`), shared by JavaScript and Python |
| Tests | chain invariants, shared vectors JS/Python, real data sets, architecture checks for layers and domains |
| Sketch mode | every app opens an infrastructure as a sketch of its own domain; the way back is the takeover ([decision 0005](docs/decisions/0005-sketch-mode.md)) |
| Documentation | one page per core module and domain, with its decisions |

See [docs/data-model.md](docs/data-model.md) for how the data is structured and stored.
