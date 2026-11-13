# Roadmap

Rail-Across-Suite is built around one shared data foundation, the **Rail-Data-Core**. Every module reads and
writes it; none keeps a private model of the railway. The roadmap therefore starts with the core and adds
modules only on top of it. The Open-Layout-Tool is the existing module and the source of the core's first
version: its project format (oltjson-1.1, replaced by infra-data-1.0 with decision 0021) already held the track network.

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
- Licences: code under Apache-2.0, documentation under CC BY 4.0 ([decision 0002](docs/decisions/0002-licences.md));
  since then everything under LGPL-3.0-only, OSRD's licence ([decision 0015](docs/decisions/0015-lgpl.md)).
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
  (now LGPL-3.0-only like everything); the values from the DB guidelines may be taken over.

Open:

- Upload the NTv2 grids as the assets of release `data-1` (until then they come from cdn.proj.org).
- The name ([decision 0007](docs/decisions/0007-project-name.md)): *Across* checked in the registers of the DPMA
  and the EUIPO before the first release.

**Exit:** the data model is reviewed, and every question left open names the phase that decides it; the
Open-Layout-Tool builds, tests and deploys from this repository.

## Phase 1 — Rail-Data-Core v1.0, extracted from Open-Layout-Tool

Goal: the core exists as a package of its own and OLT runs on it **without any change in behaviour** — what it
keeps and exchanges is infra-data-1.0 (first called core 1.0), already in the data model's shape
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
- **infra-data-1.0**, first called core 1.0 ([schemas](schemas/README.md), [document](packages/core/docs/document.md)): JSON Schemas for
  the domains `core` and `layout` with the core types `TrackEnd`, `TrackLocation`, `TrackRange`, `TrackPath`,
  `Lifespan`, `Ref`; platforms and end marks located by them, named runs of tracks as `trackPaths`, the planning
  status as lifespans over the stage list (existing, final), stations as an operational point register, bulk point
  arrays as blobs; collections sorted by id, canonical JSON after RFC 8785, a manifest of hashes per domain and
  collection; a project as one file, `.railacross.json`, carrying its schemas. The types derived from the schemas
  (`packages/core/types/document.d.ts`).
- The one door: a document is checked whole (envelope, versions, schema hashes, every object against its schema,
  blob hashes) on the way in and on the way out. oltjson-1.1 was migrated at the door — the first step of the
  migration chain —, losslessly both ways (the PEK data set was a test); since decision 0021 it is refused, and
  the format is called infra-data-1.0. Ids stay on tracks, not on their elements
  ([decision 0004](docs/decisions/0004-ids-on-tracks.md)). Files and the project server hold infra-data-1.0 documents;
  the server migrated its oltjson-1.1 revisions once (data step `0021-infra-data`). The app keeps its record as the
  model it works on until phase C of decision 0021 moves the project into the core's module.
- The shared vectors in `schemas/vectors/`: the door, canonical JSON, hashes and manifest, which the JavaScript core
  and its Rust crates are held to (a Python core held to them too was taken out with decision 0020, step 10: nothing
  used it).

Open:

- Nothing of phase 1's own. The name check (phase 0) and the release `data-1` are still to do.

**Exit:** OLT imports its model only from the core; all OLT tests and the PEK data set pass; an oltjson-1.1
file opens and saves losslessly; the cases F2 and F3 (reversal and splice) of the data model are tests and
pass. — Reached.

## Phase 2 — Generalised core

Goal: the core can carry what the planned modules need, before they are written.

The format has this phase's shape since its first version (decision 0009): domains, location types, lifespans, the
register, canonical JSON and the manifest exist as data. What this phase adds is the behaviour behind them, and
the app working on the document instead of its record.

- The app works on the infra-data-1.0 document; the record and the conversion go.
- Location types `TrackLocation`, `TrackRange`, `TrackEnd`, `TrackPath`, `Ref` (in the schema since its first version) as
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
  lifespans are in the format since its first version (oltjson-1.1's status converted when it was migrated: new =
  from the final stage, removal = up to it); the app holds and edits stages of a project's own.
  *Done* ([decision 0027](docs/decisions/0027-construction-stages-by-their-days.md)): the stages by their days —
  a track's, switch's, platform's or end mark's first and last day in service, typed for a track in the
  Layout-Tool's track editor —, a record at a stage in the core, the Layout-Tool's map at a stage chosen beside
  the working copy's bar, the construction stage connector of three ends. *Done* ([decision
  0028](docs/decisions/0028-one-model-lifespans-by-day.md), phase A): no stage list at all — a lifespan states
  days, `"open"` where the day is not known yet, and the states of a project are worked out of the lifespans,
  the civil structures and the layers' objects among them.
- The operational point register (in the format since its first version, drawn from the station texts) edited as such.
  *Done* ([decision 0028](docs/decisions/0028-one-model-lifespans-by-day.md), phase B): the record holds the points
  as objects with ids, a track and a platform name theirs; nothing is drawn from texts any more but what an
  import reads.
- Domains with ownership: `core` (net, platforms, end marks, structures, operational point register, designed
  kilometrage) and discipline domains that store references only into the core.
- Storage as a hash tree on the server and in the browser: revision → manifest → domain → collection, the blobs
  stored once (the manifest, the blobs and canonical JSON exist since the format's first version); the exchange container
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
a finding and its plane position; JavaScript and Rust give the same hashes for the shared vectors; a part of
a track rebuilt over three stages gives a valid network in every stage; a sketch with a crossover of its own is
taken over with its signals carried or orphaned in the plane, and its crossover listed as a proposal.

## Phase 3 — First new infrastructure modules

Goal: prove the core with a second real module.

- **Catenary-Tool** (`apps/catenary-tool/`, [decision 0025](docs/decisions/0025-layers-on-the-core.md)): the layer
  `electrification` — first the electrified ranges of the tracks and their systems, then masts, cantilevers,
  contact wire height and stagger, tension and feeding sections — located objects and ranges along the track,
  reading geometry, cant and heights from the core. *Done:* the app on the variant's working copy, the electrified
  ranges planned on the map — a track and two stations typed or picked, a system, a planning status —, their check
  (off the track, overlapping, two systems meeting without a section), how much of the net is under wire; the ranges
  carried over every track edit (R4); written into RailJSON. *Next:* neutral sections, masts and cantilevers.
- **Civil-Tool** (the Layout-Tool's app under `/civil/`, decision 0025): the layer `civil` — platforms, measured
  axes and reference axes from the point clouds, the plan header, the project picture —, taken out of the
  Layout-Tool, which keeps the alignment alone. *Done:* one build, the tool by its path, a bridge as its mark; the
  platforms, each making the stopping place at it a passenger stop in the core or writing one, the cross sections
  with the point clouds and surveys, Verm.ESN axes as reference axes, the plan's title block in the Civil-Tool;
  the project's settings, the topology, the runs, creating, splicing, switches, editing tracks and heights in the
  Layout-Tool.
- **Open-Signalling-Tool**: signals, detection points, balises, track sections, signalling routes, speed
  profiles; station boundaries stored as references to what marks them (entry signals, trapezoid boards, entry
  switches, platforms), with a special-case finding where they differ from EBO §4; export towards PlanPro /
  EULYNX.
  *Done:* the domain `signalling` 1.0 after PlanPro 1.10 with overlaps and flank protection, checked against the
  PlanPro plan P-Hausen; the catalogue of all Ril 301 aspects with the Ks and shunting signal kinds as logical signals after OSRD, and the balises and magnets of PZB and ETCS, in German and English; places and their logical signals, the speed signalled per route at the start signal (the design speed the core's), overlaps measured from speed and gradient (a draft of Ril 819.0202), platforms as station boundaries, the domain checked against OSRD's RailJSON; one working copy per variant in the browser for both tools; its content check
  with track sections derived over the net; the core carrying its locations on every track edit and merge (R4,
  R7); writing PlanPro plans, valid against the XSD 1.10.0.3 and packed as `.planpro`, from the infra-data document (an import was tried and went: a plan holds too little of a project, decision 0022); the server checking the domain at check-in; the Signalling-Tool's first version,
  setting objects as OSRD's editor does (snapped to a track, shown before the click) and drawing them beside their track,
  its panels with menus and pages as the Layout-Tool's — creating signals, train detection and balises apart from editing them in one overview
  ([decision 0011](docs/decisions/0011-signalling-domain.md)) on the suite's look as a library, `@rail-across/ui`
  ([decision 0012](docs/decisions/0012-ui-library.md)). *Next:* the values of Ril 819 and Ril 483, level
  crossings, block and the other parts of a station plan; crossings and slips in the PlanPro export.

**Signalling goes first**: it needs locations, ranges, functional objects and topology, and so tests the
generalised core most. Catenary and civil follow, with infra-data-2.0.

- **Noise-Study-Tool** (planned; the infrastructure editor shows it behind "more"): a noise study on the variant —
  the trains' emission from the tracks and the operations' timetables, its propagation over the terrain, the noise
  barriers and bridges of the layer `civil`; the levels at receivers and as a map, a variant against another.
- **Coordination checks** between the disciplines, derived from shared locations and storing only findings
  (signal on a mast, El signals before a neutral section, signal sight past masts).
- Decided in this phase: who writes a coordination domain, should a relation between disciplines not be
  derivable from location ([data model §8](docs/data-model.md#8-open-questions)).

**Exit:** the module works on the same project as OLT; editing tracks in OLT keeps the module's objects placed.

## Phase 4 — Operations, timetable, capacity

Goal: a project's operation on its infrastructure — timetables, their running and blocking times, and what a line
and a station carry —, computed on the variant and its construction stage as they are, with no second model of the
railway.

- **Fahrplan-Editor** of the Fahrplan-Sim (`apps/timetable-editor/`, [decision 0016](docs/decisions/0016-timetables-of-a-project.md)):
  *done:* a project's timetables on the server, laid on any of its variants; trains with number, kind, vehicle,
  departure and margin that begin and end at a station (the layout's station tracks) and stop at or pass stations
  and halts on the way, a departure waited for at a stop; their times from the running time simulation, a row for
  every station passed; copies at an interval (Takt); the graphical timetable along a train's way; the blocks of a
  train's way from main signal to main signal and their blocking times after Ril 405 — held section by section
  where the variant has track sections, with the overlaps —, the conflicts of two trains, and a dispatching that
  lets a train wait or take another route, drawn in the graphical timetable and played on the map. *Next:* times
  typed in and checked against the running times, connections and turnarounds (O4 below), the exchange with
  OSRD's timetables.
- **Running times** in the Fahrplan-Sim (*Fahrzeit rechnen* of the Fahrplan-Editor, `apps/timetable-editor/src/runtime/` — once `apps/simulation-tool/` —, [decision 0013](docs/decisions/0013-running-times-in-rust.md)):
  *done:* OSRD's envelope simulation ported to Rust (`sim/`, [decision 0015](docs/decisions/0015-lgpl.md)) and
  held to OSRD's own test cases; a train over the net of a variant — its path over switches, the grades of the
  heights and curves (arcs and transitions, after Röckl), the design speeds and the speeds the signals signal, stops at signals, platforms and points
  on the map, a margin linear or after MARECO —, run in the browser as WebAssembly on the working copy; the
  speed and the time over the distance, the timetable of its stops, the run played on the map; the vehicles as
  OSRD's RailJSON rolling stock in a catalogue of the suite, edited in the Fahrzeug-Editor
  (`apps/vehicle-editor/`), OSRD's example vehicles built in ([decision 0014](docs/decisions/0014-vehicles-in-osrds-form.md)). *Next:* the electrification of the net for
  the traction mode (infra-data-2.0 below), a train's formation in place of one vehicle (operations-1.0 below).
- **Capacity simulation** — the Kapazitäts-Sim of the start page, an app of its own (`apps/capacity-sim/`,
  [decision 0023](docs/decisions/0023-capacity-simulation-of-our-own.md)): the suite's own, on the running and
  blocking times above; OSRD run as a service (decision 0006) went. *Next:* the steps K0 to K3 below, and the two
  formats they need — operations-1.0 for what is the operation's, infra-data-2.0 for what is the railway's.

### What the capacity simulation reads, and what it lacks

| It needs | Today | Comes with |
|---|---|---|
| the way, grades and curves, speeds, stations and stops, blocks, overlaps, track sections, stages | infra-data-1.0: `core` and `signalling` | — |
| where a block is asked for, how long an overlap stays set | in the format — the distant signals, `routes[].shortDistantSpacing`, `overlaps[].releaseDelay` —, not read: a timetable's 1000 m and 60 s for every block | K0 |
| the time to set and to release a route | 12 s and 6 s, a timetable's | infra-data-2.0 |
| the speed a train may run at | the design and the signalled speed, and one vehicle's top speed | operations-1.0: the formation's top speed and brake |
| the traction system along the way | none: a train's mode is chosen for its whole run | infra-data-2.0 |
| the train protection | none | infra-data-2.0 |
| how close trains follow at a planning boundary | none: a train enters as timetabled | infra-data-2.0 |
| what a train is made of: load, length, brake | one vehicle of the catalogue | operations-1.0 |
| the kinds of trains and their rank | a list in the code, an order per timetable | operations-1.0 |
| connections, turnarounds, the least dwell | none | operations-1.0 |
| what a study assumes: period, stretches, bounds, delays, runs | none | operations-1.0 |

Timetables and vehicles are no part of an infra-data document, and stay none: a timetable is a project's, laid on
any of its variants (decision 0016), and a result is derived, never stored (data model, S-R6).

### Capacity simulation: K0 to K3

Each step answers a question of its own and is of use without the next.

- **K0 — The blocking time as the format gives it.** No format changes. A block is asked for at the signal that
  announces its own — the distant signal on the way, or the main signal before where that one announces it, a
  shortened distant spacing as the route says — in place of 1000 m for all; an overlap stays set for its
  `releaseDelay` in place of 60 s; the sight time counted apart from the time to set the route. *Done when* a
  train's blocking time stairs over a reference stretch are those worked out by hand after Ril 405, part by part.
  **The reference cases are the suite's own**, made up and kept in the repository — no real data —: a two-track
  line with Ks block signals, a single-track line with a crossing station, a station with an overtaking track;
  each with its blocking time parts, headways and compressed occupancy worked out step by step beside it, apart
  from the code, so that a test holds the code to them.
  *Done:* the block asked for within sight of the signal that announces it — a distant signal, or the main signal
  before showing Ks 2 —; an overlap's `releaseDelay`; a route's times; reference case A, a two-track line with Ks
  block signals ([docs/reference-cases](docs/reference-cases/README.md)). *Next:* the sight time of the operation's
  apart from the sight distance (operations-1.0), reference cases B and C.
- **K1 — Minimum headways and occupancy.** From a timetable's blocking time stairs: the minimum headway
  (Mindestzugfolgezeit) of the trains that follow each other on a stretch, as a table by kind; the timetable
  compressed over a stretch and a period after UIC Code 406 — the occupancy time, the occupancy rate with its
  additional times, the trains that set it —, for a line section and for the tracks and the routes of a station.
  The stretches proposed from the net — from one place where trains overtake, cross, begin or end to the next —
  and kept by their places. Computed in Rust, in a crate beside the running times, from the blocking times the
  page has. Shown in the Kapazitäts-Sim, an app of its own (`apps/capacity-sim/`, the start page's tile), on a project's
  timetables and variants as the Fahrplan-Editor has them and with the same packages: the compressed graphical
  timetable, the table of headways, the rates held against the bounds of UIC Code 406 — its recommended rates by
  kind of line and period, which a study may change —, one variant or stage beside another. *Done when* the compressed occupancy of a
  reference timetable is the one worked out by hand, and two stages of a project stand beside each other.
  *Done:* `crates/capacity` in the core's module — the minimum headway of two trains, a stretch compressed in the
  order its trains run (occupancy time, rate with and without additional times, the trains that set it), how long
  each track and section of a station is held, UIC 406's bounds —; `@rail-across/capacity` — a variant at a stage,
  the stretches proposed from where trains overtake, cross, begin or end —; the Kapazitäts-Sim (`apps/capacity-sim/`,
  `/capacity/`, the start page's tile): a timetable run over a variant at a stage, its stretches with occupancy
  against the bound, the compressed and the timetabled blocking time stairs, the headways by kind, the stations'
  tracks, another variant or stage beside it; reference case A's hour compressed by hand
  ([docs/reference-cases](docs/reference-cases/a-two-track-line.md)). *Next:* the occupancy of a station's routes
  as UIC 406 compresses a node, the stretches kept by their places (a study, O5).
- **K2 — Free train paths and saturation.** A train of a kind, a formation and a way laid into a timetable at the
  first departure in a window at which it conflicts with none — its stairs moved in time against the others';
  a stretch saturated with such trains, in a given mix, until none fits or the rate reaches its bound: what a
  stretch carries in a period beyond what is timetabled. A search of the suite's own on its blocking times, not
  STDCM. *Done when* a train laid in runs without a conflict in the dispatched timetable.
- **K3 — Operation under delays.** A timetable run many times, each time with delays drawn from a scenario — at
  the entry into the planning area by place and kind, at stops as a longer dwell; the suite proposes one: per
  kind of train a share of trains delayed and their delays drawn from a distribution, each figure a starting value
  of the suite's, shown as such and with where it comes from, for the planner to replace by measured ones —, dispatched as the timetable
  says, connections and turnarounds held (O4): the delay at the places measured at, its growth over a stretch,
  the punctuality against a threshold, the waiting of the trains for each other.
  **K3a:** the chain from a timetable to its dispatched blocking times — the way over the net, the request to the
  simulation, the run, the occupancy; under 2 000 lines of JavaScript in `packages/running-times` and
  `packages/timetables` — in Rust, held to vectors frozen from the JavaScript; a measurement first, of what a run
  costs and where it must run (decision 0021, point 6).
  **K3b:** a study as a job of the server: its runs from a seed, the same seed the same figures.
  *Done when* a study without delays gives the timetable itself, and a study repeated gives what it gave.

A result is cached under the hashes of what it was computed from — the variant's document, the stage, the
operations document, the simulation's version — and is no part of a revision.

### operations-1.0

What is the operation's, as a format of its own beside infra-data: the domain `operations` 1.0 in a document of
its own per project (`.operations.json`), with the envelope and the door of infra-data (`crates/doc`) — a schema
named by its hash, collections sorted by id, blobs by hash, references declared (`x-target`). It replaces the
`rail-across-timetables` 1.0 file, which has no schema, and the server's checks by hand.

| Collection | Holds | Today |
|---|---|---|
| `timetables` | name; the dispatching (mode, priority); the parts of the blocking time that are the operation's — the sight time, and what stands in where the infrastructure says nothing | the server's `timetable` rows |
| `trains` | of a timetable: number, kind, formation, departure, margin; its points — place, track, stop, dwell and least dwell, departure, itinerary | a timetable's `trains` |
| `trainKinds` | the kinds of trains: name, rank in the dispatching, colour | a list in the code, `dispatch.categories` |
| `formations` | Zugbildung: the traction units by their key in the catalogue and their hash; the hauled load — mass, length, top speed; brake position and braked weight percentage | a train names one vehicle |
| `vehicles` | the vehicles the formations name, as RailJSON rolling stock in blobs: the file carries what it runs with | the suite's catalogue, not in the file |
| `connections` | Anschluss: from a train's arrival to another's departure at a place; the time to change, how long it is waited for | — |
| `turnarounds` | Wende: from a train's end to the train its vehicles run as next; the least time to turn | — |
| `delayScenarios` | the delays at entry by place and kind, the longer dwells — each a distribution with its figures | — |
| `studies` | a timetable, a period, the stretches from place to place, the bounds, a scenario, the number of runs and the seed | — |

It names the railway by id — `core.operationalPoints`, `core.endMarks`, `core.tracks`, `core.stoppingPlaces`
(from infra-data-2.0; `signalling.stoppingPlaces` before), `signalling.itineraries` — and belongs to no variant: the references are checked where the operations are laid
on a variant, and one that finds nothing there is a finding at its train, not a refusal, as it is today.

The document is the whole of a project's operations — what is exported, imported and hashed for the caches — and
is kept in **revisions, as a variant's state is**: immutable, by hash, with a history and the comparison of two
revisions. A timetable saved on its own makes a new revision of the operations; a save on a stale revision is
refused, as it is today. This changes the counter per timetable of decision 0016; a record of its own says so
when O2 is built.

**A train pins its vehicles**: a formation names each by its key in the catalogue and its hash, and the document
carries each one so named. A change in the catalogue changes no computed timetable unnoticed — the train says
that its vehicle has changed and takes the new one when asked. An import whose vehicle the catalogue has under
the same key with another content takes it under its hash, beside the catalogue's, and says so.

- **O1 — The format.** The schema whole (`schemas/operations/1.0.schema.json`): a schema is a version, so the
  collections that come to life later are in it from the first day. The door reading and writing it; vectors for
  JavaScript and Rust. *Done when* both give the same hashes for the vectors.
  *Done:* `schemas/operations.schema.json` and `schemas/operations/1.0.schema.json`; the door in `crates/doc`
  (`operations`) — the shape, the references into its own collections, a carried vehicle held to its hash —, the
  timetables as the suite keeps them into a document and back (a train's kind by name, its vehicle by key, each
  drawn as an id from the name or key); vectors in `schemas/vectors/operations.json`, written by the JavaScript's
  face, held by the Rust.
- **O2 — Today's data in it.** A data step of the server for the `timetable` rows, each project's the first
  revision of its operations; the door in place of the checks
  by hand (`timetable_fault`); a `rail-across-timetables` 1.0 file still read, and turned on its way in; export
  and import as an operations file with the vehicles its trains run with. *Done when* every timetable of the
  server opens as it was, and a project exported here runs on another server with the same times.
  *Done* ([decision 0026](docs/decisions/0026-operations-in-revisions.md)): the server keeps a project's operations
  in revisions (`operations_revision`), the timetables' API reading the head and writing revisions, a save on an
  older revision taken where its timetable is unchanged; the door in place of the checks by hand; data step
  `0026-operations`; the history and each revision's document under `/api/projects/<id>/operations`; the export an
  operations file carrying its trains' vehicles, the import of it and of a file of before, a vehicle the catalogue
  lacks or has otherwise made in it beside its own.
- **O3 — Formations in the simulation.** The mass, length, top speed and brake of the whole train in the request:
  a train runs no faster than the line, its signals and its formation allow — a freight train's lower speed is
  its own, not the line's. Its deceleration follows from its brake position and braked weight percentage by
  ETCS's conversion model (UNISIG Subset-026), as a deceleration by speed; the simulation, which brakes with one
  constant deceleration today (`const_gamma`), takes it in steps of speed — an extension of the port, held to the
  conversion model's figures. ETCS's braking curves stay unported.
- **O4 — Connections, turnarounds, least dwell.** Held by the dispatching; a connection missed and a turn too
  short as findings at the train.
- **O5 — Studies and scenarios.** Edited in the Kapazitäts-Sim, run by K1 to K3.

### infra-data-2.0

The layers of [decision 0025](docs/decisions/0025-layers-on-the-core.md) and what the simulation needs of the
railway itself, in the infrastructure's format. Planned as 1.1; it is 2.0 because the core and signalling change
in ways a reader of 1.0 cannot leave unread (`schemas/README.md`). All of it in one version, with one step at the
door (S-R10): an infra-data 1.0 document reads in as 2.0 with everything it held.

**The envelope's version is the version of the set**: a minor one where every domain's change is an addition, a
major one where any domain takes a major one. infra-data-2.0 is `core` 2.0, `signalling` 2.0, `civil` 1.0 and
`electrification` 1.0; `layout` 1.0 goes into `civil`.

| Domain | Change | For |
|---|---|---|
| `core` 2.0 | `platforms` leave it, for `civil`; `trackPaths` from `layout` (the Layout-Tool's runs), `stoppingPlaces` from `signalling`, written by the Signalling-Tool ([decision 0024](docs/decisions/0024-every-module-writes-the-core.md)), with `passengerStop` where the Civil-Tool puts a platform at one | the core is the alignment and what every layer places things by |
| `core` 2.0 | `endMarks[].headway`: how close trains follow at a planning boundary | trains entering from a net that is not in the model |
| `civil` 1.0, new | what `layout` 1.0 held — `axisSurveys`, `referenceAxes`, the plan header, the project picture, the RailJSON objects an import brought along — and the `platforms`, each naming the stopping places at it (`stoppingPlaceIds`) | the Civil-Tool's; a platform put makes the stopping place at it a passenger stop (`passengerStop`, core), or writes one |
| `electrification` 1.0, new | `electrifications`: a range of a track, its system, a lifespan | the traction mode along a train's way; where there is none, no wire — the Catenary-Tool's |
| `signalling` 2.0 | `stoppingPlaces` leave it, for the core; its itineraries name them there; a station boundary marked by a platform is marked by the stopping place at it (`stoppingPlaceId`) | no stored reference between sibling domains (R9) |
| `signalling` 2.0 | `routes[].settingTime` and `.releaseTime`, with the interlocking's values in the domain's fields | the blocking time's parts that are the interlocking's |
| `signalling` 2.0 | `trainProtection`: a range of a track, the system and its level — PZB, LZB, ETCS —, a lifespan | the braking a train is run with; later, blocks without signals at the track |

A track's `lagesystem`, `proviAxis` and `osrd` stay with it in the core: the Layout-Tool's imports of the alignment
write them.

Not in 2.0: speeds for a kind of train — the line's speed is the design speed and what the signals signal, for
every train; a speed profile of its own, as PlanPro's `Geschwindigkeitsprofil` (tilting trains, ETCS's train
categories), comes with a line that needs one, with the train protection —; level crossings (the
Signalling-Tool's next, phase 3), neutral sections (with the catenary), anything of the operation, any result.

- **I1 — The format.** `schemas/core/2.0.schema.json`, `schemas/signalling/2.0.schema.json`,
  `schemas/civil/1.0.schema.json` and `schemas/electrification/1.0.schema.json` beside the 1.0 files; the door's step
  from 1.0 — `layout` and the platforms into `civil`, the stopping places into the core, a stopping place's
  `platformId` into its platform's `stoppingPlaceIds`, a boundary's platform mark into the stopping place at it (one
  made over the platform where there is none); the additions begin empty —; the vectors and types written again;
  the server's revisions brought to 2.0 by a data step. *Done when* every project of the server opens with every
  platform, stopping place and boundary in its place, and a file exported as 1.0 reads in with what it held.
  *Done:* all of it, held to vectors of infra-data 1.0 documents and to the server's data step `0025-infra-data-2.0`;
  a layer that holds nothing is written as missing.
- **I2 — The tools.** The Layout-Tool the alignment's alone; the Civil-Tool — the Layout-Tool's app under
  `/civil/` — for the platforms and the passenger stops, the surveys and reference axes, the plan header and the
  picture (the runs stay the Layout-Tool's, in the core); the
  Catenary-Tool (phase 3) for the electrification; a route's times and the train protection in the
  Signalling-Tool; a boundary's headway in the Layout-Tool. *Done when* each tool edits its own layer and shows the
  others read-only, and a range keeps its place over a track split, joined and reversed.
  *Done:* the Layout-Tool, the Civil-Tool and the Catenary-Tool as above; a route's times and the interlocking's
  in the Signalling-Tool, which shows a station's platforms read-only; a boundary's headway. *Next:* the train
  protection in the Signalling-Tool (kept through its edits, not edited yet).
- **I3 — Reading.** The running time takes the traction mode from the electrification along its way; the
  blocking time takes a route's times; a train enters at a boundary no closer than its headway; RailJSON's
  `electrifications`, empty in the export today, written and read.
  *Done:* the mode a train runs with chosen by the wire along its way where the train names none — one mode the
  whole way, as OSRD's switching between modes is not ported —, and where it lacks, said; a route's times in the
  blocking time; a train entering a boundary too soon after the one before, said (it is not held back yet);
  RailJSON's electrifications written — read, they stay what the import brought along, as the Catenary-Tool plans
  the layer.

### Order

1. **I1, I2** — the layers of decision 0025 on infra-data-2.0, with the Civil-Tool and the Catenary-Tool. *Done*,
   but the train protection's editor.
2. **K0** — the blocking time as the format gives it; every later figure rests on it. *Done*, but the operation's
   sight time and reference cases B and C.
3. **O1, O2** — the operations on their format before anything new is put into them. *Done.*
4. **K1** — the first figures, from what is there. *Done*, but a station's routes compressed as a node.
5. **I3** — the running and blocking times read the railway's additions. *Done* ahead of its place, but holding a
   train back at a boundary.
6. **O3, O4** — formations, connections and turnarounds.
7. **K2**.
8. **O5 and K3**.

K1 gives its first figures with neither format's additions; K2 and K3 are as good as the formations, the
electrification and the connections make them.

### Open

- **The project picture once**: `image` (a data URL in a file) and `imageHash` (a blob on the server) are two ways
  for one picture; one, as a blob of the file, proposed.

**Exit:** for a stretch of a project, the headways, the occupancy rate, the trains it still carries and its
delays under a scenario come from the variant and the stage as planned, are the same for the same inputs, and
are the reference cases' where there is one.

## Phase 5 — Construction simulation

Construction stages and lifespans are part of the core from phase 2
([decision 0003](docs/decisions/0003-construction-stages.md)); this phase builds on them.

*Begun:* the Construction-Sim (`apps/construction-sim/`, `/construction/`, the start page's Baubetriebs-Sim): a
variant's net stage by stage — the existing state, every day something is put into or taken out of service,
the final state —, what comes into service with each and what leaves it, the stage connectors
([decision 0027](docs/decisions/0027-construction-stages-by-their-days.md)).

- `construction` domain: closures, temporary speed restrictions and the tracks in operation per stage.
- The stages' periods and the work between two stages — what ends and begins there — as the base of the
  construction sequence; activities, durations and resources.
- Every stage checked as a network with the same validation, signalling and, later, the capacity simulation.

## Cross-cutting, in every phase

| Topic | Direction |
|---|---|
| Storage and server | one project server for all modules: users, projects, variants and sketches, immutable revisions, merge in the browser, validation on check-in, approvals pinned by hashes and checked by the diff (data model S-R6) |
| Exchange | RailJSON, railML 3, IFC 4.3, PlanPro, LandXML, Verm.ESN — translations at the edge, never the working format |
| Rules | rule and form catalogues as data (as OLT's `constraints/`), shared by JavaScript and Rust |
| Tests | chain invariants, shared vectors JS/Rust, real data sets, architecture checks for layers and domains |
| Sketch mode | every app opens an infrastructure as a sketch of its own domain; the way back is the takeover ([decision 0005](docs/decisions/0005-sketch-mode.md)) |
| Documentation | one page per core module and domain, with its decisions |

See [docs/data-model.md](docs/data-model.md) for how the data is structured and stored.
