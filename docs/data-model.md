# Rail-Data-Core — data model proposal

Status: **proposal, under review** — hardened against concrete cases (section 2). It is the design input for
phases 1 and 2 of the [roadmap](../ROADMAP.md).

Every module of Open-Rail-Designer reads and writes one shared description of the railway. This page proposes
how that description is structured and stored. It starts from what already works, the project format of the
Open-Layout-Tool, **oltjson-1.1** (`open-layout-tool/docs/data-format.md`), and generalises it where the
planned modules need more than a track layout.

Section 2 tests the model against concrete cases; sections 3 to 5 give the rules that follow from them.

## 1. What oltjson-1.1 gets right — and keeps

| Principle | Why it matters for every module |
|---|---|
| **Design values, not polylines.** A track is a chain of straights, arcs and transitions with length, radius, cant. | A mast, a signal or a running-time calculation needs the true curvature and cant, not vertices. |
| **Derived data is never stored.** Polylines, symbols, polygons are rebuilt on load and stripped on save. | What is not stored cannot disagree. Diffs and merges see only real changes. |
| **Stable UUIDs, references by id** (plus `BEGIN`/`END` for track ends), never names, never array indices. | Modules reference each other's objects across revisions, variants and merges. |
| **Collections keyed by id.** | Field-by-field three-way merge. |
| **Plane coordinates, one plane per track**, joined by two-end connectors (`link`) where the plane changes or a rebuilt part ties in. | Survey data stays exactly as delivered. |
| **Stations along the track** for whatever is placed on it. | Survives re-shaping of the horizontal alignment. |
| **One door.** One parse/validate function decides what is accepted; everything behind it trusts the model. Where oltjson refuses an older format, the core migrates it there, explicitly and tested (S-R10), and refuses only what cannot be migrated. | No "field might be missing" scattered through six modules. |
| **Shared validation** in browser and server; only *new* errors block. | Imported data with real defects stays workable. |
| **Precision with a reason**: positions, lengths and stations to 1 µm, heights to 0.1 mm, bearings to 10⁻⁹° ([conventions](conventions.md)). | A turnout laid into a track read back at 0.1 mm would inherit a tangent break ten times what the chain checks allow; a rounded bearing walks a long track off course. |

## 2. The cases

Each case names what happens, what it breaks, and the rule it leads to. Cases marked **tested** were run against
the Open-Layout-Tool's code as of 2026-10-29; the defects they found are fixed in phase 1 (section 6). The test
files are kept outside the repositories.

### Cases about references

**F1 — A turnout is laid into a track that carries signals and masts.** The layout tool splits track `T` into
`T1` and `T2`; `T` is gone. The signals and masts on `T` belong to the signalling and catenary domains, which the
layout tool does not understand. If it kept the domains it does not understand unchanged, they would now name
a track that does not exist.
→ The core must carry references in **every** domain, including those the running module does not know. It can
do so only if it can *find* them: references are declared in each domain's schema through core types (rules
R3, R4).

**F2 — Concurrent edit: Alice splits a track, Bob moves a platform on it. (tested)** Base: platform `P` on `T`
at 120–260 m. Alice splits `T` at 100 m; her step re-stations `P` to `T2` at 20–160. Bob, on the old state, moves
`P` to 135–275 on `T`. The Open-Layout-Tool's merge (as of October 2026) compares field by field: it takes `trackId = T2` from Alice and reports a
conflict on both stations, although Alice never touched the platform. Whichever side is chosen, the result is
wrong: Bob's side gives `T2` at 135–275, **100 m off** and still within the track, so no check notices; Alice's
side loses Bob's move.
A prototype that first carries base, mine and theirs through both id logs and then compares gives `T2` at
35–175 without a conflict, in both merge directions.
→ Carry first, then compare (R6); a location is one value (R5).

**F3 — A track is reversed; two tracks are spliced. (tested)** In the Open-Layout-Tool (as of October 2026),
`reverseTrackDirection` flips the switch ports and end marks of the track, but not its platforms: a platform at
120–260 m on the left of a 400 m track stays at 120–260 on the left — counted from the other end now, so it has
moved 20 m and changed sides. Splicing two tracks into one ("Zwei Gleise zusammenfügen") carries switch ports and
end marks onto the new track, but a platform keeps naming the old track, which is gone.
Every object kind carries its own remap code there, and in two places one was forgotten. With signals, balises,
masts and speed boards the number of such places would multiply.
→ One location type, one remap, owned by the core (R4).

**F4 — A track is deleted on which another discipline's objects stand.** Deleting means the track never existed:
it was drawn or imported by mistake. A track taken out of service is not deleted but removed — its lifespan ends
(`to:`), and containment applies (R13, case F11). Within its own data the layout tool cascades a deletion (end
marks and platforms go with the track). Deleting twelve masts of the catenary planner the same way would destroy
work its owner never saw.
→ Never delete another domain's work; orphan it with a finding, and warn before (R7). The warning needs to know
what stands on the track, which only the domains above know → reverse index, derived (R8).

**F5 — A signal hangs on a catenary mast; El signals stand before a neutral section; masts must not block the
sight of a signal.** Signalling and catenary each need the other: references in one direction only cannot
express that, and storing them both ways makes a cycle.
All three relations are positions on the same track: same station and side, a distance before a range, a
sight line along the track. They can be **derived** from the shared location, like the Open-Layout-Tool derives
which track ends meet instead of storing it.
→ No stored references between sibling domains; coordination checks derive and check these relations (R9).

**F6 — Station Halle-Trotha.** In oltjson, station tracks and platforms name their station by text
(`stationName`, `uicStation`, `code`). Operational points can therefore not sit in a layer above the net: tracks
and platforms refer to them, so such references would point upward. Moreover, an operational point has an
identity (name, RL100 code, UIC number) and boundaries. By EBO §4 a station's boundaries are, in general, its
entry signals or trapezoid boards, otherwise its entry switches; a halt has no entry signals, its extent is in
practice its platform. Special cases differ from that rule. A new halt has no RL100 code yet.
→ Identity and boundaries are two facts with two owners (R10): an operational point register in the core with
UUIDs (codes are attributes, added when they exist); the boundaries **stored** in the signalling domain, so that
special cases can be stated — each as a reference to what marks it (an entry signal or trapezoid board, an entry
switch, a halt's platform), never as a position of its own. A boundary that differs from the general rule is a
special-case finding with a justification, as the rule catalogue handles special cases.

**F7 — Kilometrage of a new line.** Kilometrage looks like reference data below the net, and for an existing
line it is (DB km lines). For a new line it is defined *by* the designed axis: a start km on a run of tracks plus
km jumps.
→ Designed kilometrage lives next to the tracks in the core, not below them. What holds is not a stack of
layers but an acyclic graph of domains (R2).

**F8 — Speed.** The rule checks of the layout tool use the design speed on each element. Signalling plans the
permitted speed (speed boards, speed profile). If the layout check read the permitted speed, the layout would
depend on signalling, which depends on the layout.
→ Two facts, two owners (R10). The check "permitted speed ≤ what the geometry allows" belongs to signalling, the
domain that reads both.

**F9 — An existing mast, and the track is shifted by 0.5 m.** A planned mast should follow the track. A mast that
stands on its foundation does not move because a design line moved; the change should instead raise a
clearance finding.
→ A location states its anchor, and existing objects are anchored in the plane (R11).

**F10 — An older build opens a project with a newer domain.** A signalling 1.2 domain moves its location
fields; the layout tool built against signalling 1.0 (or not knowing signalling at all) splits a track.
→ The schema travels with the data, so references can be found in any domain version (R3). A build that meets a
location type it does not know refuses the edit, rather than leaving references behind (R4).

**F11 — A part of a track is rebuilt in stage 3, and a signal stands on the old part.** The track is split at the
tie-in points; the old part ends in stage 3 (`to: 3`), the new part begins there, each joined by a connector of
its own ([decision 0003](decisions/0003-construction-stages.md)). The signal on the old part would outlive its
track: from stage 3 on it would stand on nothing. The layout tool must not end or move it — it is signalling's
work.
→ Containment (R13): an object exists only while what it refers to exists. What would outlive its target is a
finding for its owner, who ends the signal with its track or puts a new one on the new part that `replaces` it.

**F12 — A signalling sketch with an extra crossover is taken over.** In a sketch
([decision 0005](decisions/0005-sketch-mode.md)) the signalling planner adds a crossover to try a concept and puts
signals on it. The integrated model has no such tracks. Merging the sketch would bring its tracks along; leaving
its signals behind would lose the work.
→ A sketch returns only through a takeover limited to its scope domain (R14): signals on tracks that descend from
the same base are carried through the id logs; signals on the sketch's own tracks arrive orphaned with their
plane position, as warnings (R7); the crossover is listed as a proposal for the layout tool.

### Cases about storage

**S1 — Editing a platform stores two kilometres of surveyed axis again.** A reference axis keeps a point every
centimetre, at most 2 km: about **1.3 MB of JSON and 20–200 kB gzipped** (measured on synthetic data: smooth vs.
±1.5 mm survey noise). The whole network of the PEK Halle–Könnern data set (22 tracks, 13 switches) is ~15 kB
gzipped. Stored as one document per domain, every platform edit would store the axis anew — as
oltjson stores the whole record per revision today.
→ Address storage per collection, not per domain; bulk point arrays as blobs of their own (S-R1, S-R2).

**S2 — The optimizer and the browser hash the same data differently. (tested)** Python writes `100.0`, `1e-07`,
`-0.0`; JavaScript writes `100`, `1e-7`, `0`. Content addressing needs identical bytes from both.
→ Canonical JSON after RFC 8785, rounding first, hashing the uncompressed bytes (S-R3).

**S3 — The signalling plan is approved; then a track is shifted by 5 cm.** The signalling document did not
change, so a hash of it alone says "still approved" — yet its meaning changed. The other way round, the hash of
the tracks changes with every edit of any track, also with a temporary track in stage 7 far from what was
approved.
→ An approval pins the domain's hash *and* the hashes of what it depends on: equal hashes answer "nothing below
changed" at once. Whether a change *affects* the approval is decided by the diff: it does when changed objects
exist in the approved stages and lie in the approved area (S-R6).

**S4 — A timetable viewer and a track editor.** The viewer only needs the core and the operations domain.
The track editor must carry every reference into the net (F1), so it needs every domain that has one.
→ Partial loading is for read-only use; editing the core loads all referencing domains (S-R7).

**S5 — A track planner without signalling rights lays a turnout into a track with signals.** The check-in
changes the signalling domain: the signals' references are carried onto the new tracks. The server must accept
that but nothing more. Only the layout tool writes the core, so only track planners cause such changes; the other
disciplines place their objects by location and never split a track.
→ A changed domain that the author may not write must equal the base carried through the id log, which the
server recomputes (S-R5) — the Open-Layout-Tool already checks measured axes against a right in this way.

**S6 — Twelve construction stages.** As variants (the Open-Layout-Tool's decision), the existing and the final
state exist once per stage: a survey correction of the existing state means twelve merges, and a stage plan
would have to refer to variants from outside the revisions.
→ Stages are lifespans on one state ([decision 0003](decisions/0003-construction-stages.md), R12, R13): the
record holds existing, final and temporary objects once; a stage is a view; the stage list is part of the record.

**S7 — A capacity simulation result.** It follows from the timetable, signalling and core, for one construction
stage.
→ A cache keyed by the hashes of its inputs and the stage, never part of a revision (S-R6).

**S8 — Alice and Bob change the same track at once.** Alice changes the radius of one arc, Bob the cant of another
arc of the same track. Elements have no ids, so a track's `elements` are one value in the merge: a conflict, one
side is taken, the other change is made again. A track is edited by one person at a time, so this is rare.
→ Ids on tracks, not on their elements ([decision 0004](decisions/0004-ids-on-tracks.md)).

## 3. Rules for references

| # | Rule | Cases |
|---|---|---|
| R1 | Every collection belongs to one **domain**; a module writes only the domains it owns. The exceptions are the core library's own changes in every domain: carrying references (R4) and moving orphans to their plane position (R7). | — |
| R2 | A domain stores references only into itself or into a declared dependency; the **dependency graph is acyclic**. Inside a domain references may go any way (tracks ↔ switches, as today). | F7 |
| R3 | **References are declared, not guessed.** A schema marks every reference with a core type: `TrackLocation`, `TrackRange`, `TrackEnd`, `TrackPath`, `Lifespan` (stage ids from the core's stage list) or `Ref` (a bare UUID; the schema names its target type). The schema travels with the data (by hash in the manifest). | F1, F10 |
| R4 | **One remap.** Whenever the net changes ids or stations (split, join, reverse, delete, a length change before an object, a plane change) or the stage list loses a stage (lifespans that name it move to the following stage), the core carries every declared reference in every domain, in the same step, and records the changed ids in the id log. No module writes its own remap. A build that cannot carry a domain refuses the edit. | F1, F3, F10 |
| R5 | **A location is one value**: compared, merged and carried whole, never `trackId` and `station` separately. | F2 |
| R6 | **Merge: carry first, then compare.** Base, mine and theirs are carried into the newest ids through both id logs before the three-way comparison, so a change that is only carrying is no change. | F2 |
| R7 | **Deleting never deletes another domain's work.** Deleting means an object never existed; taking it out of service is a removal, i.e. the end of its lifespan (R12), and containment applies (R13). Within its own domain a module cascades a deletion. Objects of other domains are left orphaned, with a finding for their owner, and the user is told beforehand what stands there. **An orphan keeps its place in the plane:** in the same step the core turns its location into the anchor `plane` (easting, northing, EPSG, direction as a bearing) without a track, since a station means nothing without its track. Id and every reference to it stay. An orphan is a warning, not an error; it is attached to a track again by its plane position — within a tolerance, to exactly one track, confirmed by the user. | F4, F12 |
| R8 | **Upward is derived.** "What stands on this track" is a reverse index built on load from the declared references, never stored. | F4 |
| R9 | **No stored references between sibling domains.** Relations between disciplines are derived from their shared locations and checked by **coordination checks**, which read several domains and store nothing but findings. A relation that cannot be derived from location goes into a coordination domain above both. | F5 |
| R10 | **One fact, one owner.** If two domains seem to need the same fact, it is two facts (design / permitted speed, station identity / boundaries), or it moves into the core. | F6, F8 |
| R11 | **A location states its anchor**: `station` (follows the track), `km` (keeps its kilometrage) or `plane` (stays where it is). The anchor's value is stored, the others are derived. Objects drawn in a tool default to `station`; existing, imported objects to the form they were delivered in (plane or km). | F9 |
| R12 | **Objects have a lifespan** `{ from, to }` over the ordered construction stages of the core; the planning status (existing, new, removal, temporary) is derived from it. A stage is a view; every stage is checked as a network, and its errors count like all others (only new ones block). Objects do not change between stages: a change is a replacement that names its predecessor in `replaces`. The stage list is part of the core and written by the layout tool; the `construction` domain adds sequences and closures that refer to the stages. | S6 |
| R13 | **Containment.** An object exists only while what it refers to exists (a connector while its tracks do, a signal while its track does). What would outlive its target is a finding for its owner, never changed for them. | F11, S6 |
| R14 | **Sketches return only through a takeover.** A sketch holds the core and one scope domain and may change its core freely; it is never merged back. A takeover merges the scope domain's objects, all or selected, into a target variant: carried through the id logs where tracks descend from the same base, orphaned with their plane position where they stand on the sketch's own tracks. Changes the sketch made to the core are never taken over; they are listed as proposals for the layout tool. In the layout tool the scope is the core and `layout`, and the takeover is the regular merge ([decision 0005](decisions/0005-sketch-mode.md)). | F12 |

Findings caused by another domain's change (an orphaned signal, a mast now too close) do not block that
change's check-in; they are listed for the owner of the affected domain. A check-in is blocked only by new errors
in domains its author changed beyond carrying.

## 4. Structure

A stack of layers — reference data, net, located objects, functional objects, operations — cannot be cut
cleanly (F6, F7). The structure is a core that everyone depends on, discipline domains that depend only on the
core, and derived results above them:

| Level | Domain | Written by | Content |
|---|---|---|---|
| derived | — | — | stage views, coordination checks, simulation results, reverse index — caches keyed by input hashes, never in a revision |
| discipline | `layout` | open-layout-tool | surveys, reference axes, named track paths, plan header |
| discipline | `signalling` | open-signalling-tool | signals, detection points, track sections, signalling routes, LST switch elements, station boundaries, speed profile |
| discipline | `catenary` | open-catenary-tool | masts, cantilevers, wires, sections, El signals |
| discipline | `operations` | open-timetable-tool | train runs, timetable, rolling stock |
| discipline | `construction` | open-construction-sim | closures, temporary speed restrictions, tracks in operation per stage |
| base | `core` | open-layout-tool | **reference**: coordinate systems, km lines and designed kilometrage, operational point register, catalogue versions (rules, switch forms, profiles) · **net**: tracks (elements, heights, cant, superstructure), switches and connectors · **context**: platforms, end marks, structures (bridges, tunnels, level crossings) · **stages**: the ordered construction stages · **types**: `TrackLocation`, `TrackRange`, `TrackEnd`, `TrackPath`, `Lifespan`, `Ref` |
| project | — | server | variants and sketches, approvals, comments — they refer into revisions, never back |

Discipline domains store references only into the core (R2, R9). Open-Capacity-Sim writes no domain: its results
are derived.

- **Connectors with two ends** (`link`) join two track ends at a node: where the plane changes, and where a
  rebuilt part of a track ties in ([decision 0003](decisions/0003-construction-stages.md)).
- **The core is written by the layout tool** (and carried by the core library, R4). Platforms, end marks and
  structures are in the core because several disciplines place things relative to them.
- **One physical thing, several discipline objects.** A turnout is a core switch (geometry, form, topology); the
  signalling domain has its own switch element referring to it (drive, position detection, interlocking name),
  as PlanPro models it. Each discipline names it in its own terms; the core switch carries the geometry once.
- **Words are reserved per meaning.** In the Open-Layout-Tool "route" names a named run of tracks (`routes`) and
  the two ways through a turnout; signalling will need it for the signalling route (Fahrstraße). A bare "route"
  is therefore reserved for signalling. A run of tracks is a `TrackPath`, a core type; the layout tool's named
  runs become `trackPaths` in the `layout` domain; the ways through a turnout keep their compound name
  `switchRoute`.

A located object, here a new signal, with its location and its lifespan as one value each:

```json
{
  "id": "5d1c…", "kind": "main", "name": "N3",
  "at": { "trackId": "a9f5…", "station": 812.400, "side": "right", "offset": 2.90,
          "direction": "down", "anchor": "station" },
  "lifespan": { "from": "c41e…" }
}
```

The signalling schema declares `"at": { "$ref": "core#/$defs/TrackLocation" }` and
`"lifespan": { "$ref": "core#/$defs/Lifespan" }`. Any build finds both references through the schema: it carries
`at` through a split, can list the signal as standing on track `a9f5…`, and derives from `lifespan` that the
signal is new from stage `c41e…` on — drawn red.

## 5. Storage

### 5.1 Options

| Option | Strengths | Weaknesses |
|---|---|---|
| **A. One JSON document per project** (oltjson, as of 2026) | simple; diff and merge are pure functions; works offline | every revision stores everything (S1); one schema version for all modules |
| **B. Documents per domain, content-addressed per collection** | unchanged collections cost nothing; per-domain rights, versions and approvals | manifest, hash store and canonical bytes to maintain |
| **C. Relational DB, normalised** | queries across projects, spatial indexes | variants and three-way merge become hard; a second model for offline browsers; tempts to store derived geometry |
| **D. Operation log / CRDT** | live multi-user editing | geometric constructions (splice, turnout insertion) are not field operations; validating a state needs the state anyway |

### 5.2 Recommendation: B

A project state is logically a set of **domain documents** (unit of ownership, schema and version). Physically it
is a **hash tree**:

```
revision   { parents, author, time, message, manifest: sha256:…, idLog: [ { from, to: [ids] } ] }
   └─ manifest
        ├─ core        { version: "1.0", schema: sha256:…, fields: sha256:…,
        │                collections: { tracks: sha256:…, switches: sha256:…, platforms: sha256:…, … } }
        ├─ layout      { version: "1.0", schema: sha256:…, dependsOn: ["core"],
        │                collections: { referenceAxes: sha256:…, … } }
        │                   └─ a reference axis names its point arrays: { points: "sha256:…" }
        └─ signalling  { version: "0.3", schema: sha256:…, dependsOn: ["core"], collections: { … } }
blobs: canonical JSON of each collection · bulk point arrays · schemas · images · (point clouds as today)
```

| # | Rule | Cases |
|---|---|---|
| S-R1 | The unit of **storage** is the collection; the unit of **ownership, schema and version** is the domain; the unit of **merge** stays the object and field (R5, R6); the **revision** (one manifest) is the atomic unit — a step that changes several domains is one revision. | S1 |
| S-R2 | Bulk arrays (reference axes, axis surveys, later measurement runs) are blobs referenced by hash from their object. Point clouds and images stay outside, as today. | S1 |
| S-R3 | **Canonical bytes**: numbers rounded to the field's precision first, then serialised after RFC 8785 (JCS); set-like collections sorted by id, sequences (`elements`, `heights`) kept in order (the schema says which). The hash is SHA-256 of the uncompressed bytes; compression is a storage detail. Shared test vectors for JavaScript and Python. | S2 |
| S-R4 | Revisions are immutable; variants are named heads; the browser merges, the server appends and checks (as in the Open-Layout-Tool). | — |
| S-R5 | **The server checks per domain**: the author's right for every changed domain; a changed domain the author may not write must equal the base carried through the id log (R4), with its orphans moved to their plane position (R7). A domain whose own and dependency hashes are unchanged is not validated again. | S5 |
| S-R6 | **Derived results are caches keyed by input hashes** (coordination findings, simulation results, reverse index), never in a revision; a simulation result also by its construction stage. An **approval** pins a domain hash plus the hashes of its dependencies; whether a later change affects it is decided by the diff, restricted to the approved stages and area. | S3, S7 |
| S-R7 | **Partial loading is for reading.** Editing the core loads every domain that references it — except in a sketch, which holds no other domains (S-R11). | S4, F12 |
| S-R8 | **Project level, outside revisions**: variants and sketches, approvals, comments. They refer into revisions; revisions never refer to them. | S3, F12 |
| S-R9 | **Exchange**: a zip container with the manifest, the canonical collection files, schemas and blobs. A single-file JSON for small projects holds the same canonical documents and gives the same hashes. The file extension is still open: `.ord`, the working name, already stands for Bentley's OpenRoads and OpenRail Designer in civil design. | — |
| S-R10 | **Versions**: each domain has its own version; the one door runs an explicit, tested migration chain per domain. Behind the door only the current version exists. | F10 |
| S-R11 | **Sketches** are variants marked as sketch, with their scope domain and the revision they start from; their revisions hold the core and the scope domain only, so with hashes per collection the copied core costs nothing. The server refuses to merge a sketch into a regular variant; a takeover is checked in on the target like any change. A sketch is visible to its author until shared with the project's members. | F12 |

Server: SQLite (`revision`, `manifest`, `blob`) until several instances or cross-project queries are needed, then
Postgres with the same tables. Spatial search across projects is a derived index, never the source of truth.

Browser: working copies in IndexedDB with the same hash tree (manifest and blobs), each with its base revision
and its id log, as in the Open-Layout-Tool. The local build without a server keeps its projects the same way.
Point clouds stay in the browser's private file system (OPFS) or on the server, outside the record.

## 6. Consequences for the Open-Layout-Tool

**Defects** the cases found in its code as of October 2026, fixed in phase 1 of the roadmap:

1. **Merge after a split (F2):** a platform moved on a track that the other side split ends up off by the
   distance to the cut (100 m in the example), or loses the move. Fix: carry all three states first, then
   compare; merge a platform's track, stations and side as one value.
2. **Reversal (F3):** platforms are not carried when a track is reversed. Fix: carry platforms like end marks.
3. **Splice (F3):** platforms on the two spliced tracks keep naming a track that is gone. Fix: carry them onto
   the new track like end marks.

In the core, all three become one remap for every location (R4).

**Model changes** the Open-Layout-Tool goes through in phase 2, each brought to existing data by a step of the
migration chain at the one door (S-R10):

| oltjson-1.1 | Core after phase 2 |
|---|---|
| `stationName`, `uicStation`, `code` as text (F6) | references to the operational point register |
| `status`: `existing`, `new`, `removal` | `lifespan`: none, `from` the final stage, `to` the final stage; the status is derived (R12) |
| `routes`, named runs of tracks | `trackPaths` in the `layout` domain, of the core type `TrackPath` |
| `link` only where the plane changes | two-end connector also within one plane, for rebuilt parts (decision 0003) |
| platform and end mark fields of their own | the core location types `TrackRange` and `TrackEnd` |
| one record for everything | the `core` and `layout` domains in the hash tree (section 5) |

## 7. Open questions

1. *Answered:* Rail-Data-Core lives in this repository together with the modules, since R3, R4 and S-R10 need
   all modules on the same core version ([decision 0001](decisions/0001-one-repository.md)).
2. *Answered:* signalling is the first new module; it tests locations, ranges, functional objects and
   topology at once.
3. *Answered:* ids on tracks, not on their elements; a track is edited by one person at a time
   ([decision 0004](decisions/0004-ids-on-tracks.md)).
4. *Answered:* construction stages are lifespans on one state, not variants
   ([decision 0003](decisions/0003-construction-stages.md)).
5. *Answered:* for now changes to the core schema are approved by @serigalan, the code owner of `packages/`
   and `schemas/`.
