# Rail-Data-Core — data model proposal

Status: **proposal, second draft** — hardened against concrete cases (section 2). It is the design input for
phases 1 and 2 of the [roadmap](../ROADMAP.md).

Every module of Open-Rail-Designer reads and writes one shared description of the railway. This page proposes
how that description is structured and stored. It starts from what already works, the project format of the
Open-Layout-Tool, **oltjson-1.1** (`open-layout-tool/docs/data-format.md`), and generalises it where the
planned modules need more than a track layout.

The first draft proposed five layers with "references only downward" and "one JSON document per domain". Both
ideas survive, but section 2 shows where each of them breaks as first stated: the layers were cut in the wrong
places, "downward" alone does not say who repairs a reference, and a whole domain is too coarse a unit to store.
Sections 3 and 4 give the rules that follow from the cases.

## 1. What oltjson-1.1 gets right — and keeps

| Principle | Why it matters for every module |
|---|---|
| **Design values, not polylines.** A track is a chain of straights, arcs and transitions with length, radius, cant. | A mast, a signal or a running-time calculation needs the true curvature and cant, not vertices. |
| **Derived data is never stored.** Polylines, symbols, polygons are rebuilt on load and stripped on save. | What is not stored cannot disagree. Diffs and merges see only real changes. |
| **Stable UUIDs, references by id** (plus `BEGIN`/`END` for track ends), never names, never array indices. | Modules reference each other's objects across revisions, variants and merges. |
| **Collections keyed by id.** | Field-by-field three-way merge. |
| **Plane coordinates, one plane per track**, links where the plane changes. | Survey data stays exactly as delivered. |
| **Stations along the track** for whatever is placed on it. | Survives re-shaping of the horizontal alignment. |
| **One door.** One parse/validate function decides what is accepted; everything behind it trusts the model. | No "field might be missing" scattered through six modules. |
| **Shared validation** in browser and server; only *new* errors block. | Imported data with real defects stays workable. |

## 2. The cases

Each case names what happens, what it breaks, and the rule it leads to. Cases marked **tested** were run against
the Open-Layout-Tool's code (branch of 2026-10-29); the test files are kept outside the repositories.

### Cases about references

**F1 — A turnout is laid into a track that carries signals and masts.** The layout tool splits track `T` into
`T1` and `T2`; `T` is gone. The signals and masts on `T` belong to the signalling and catenary domains, which the
layout tool does not understand. If it "keeps unknown domains unchanged", as the first draft said, they now name
a track that does not exist.
→ The core must carry references in **every** domain, including those the running module does not know. It can
do so only if it can *find* them: references are declared in each domain's schema through core types (rules
R3, R4).

**F2 — Concurrent edit: Alice splits a track, Bob moves a platform on it. (tested)** Base: platform `P` on `T`
at 120–260 m. Alice splits `T` at 100 m; her step re-stations `P` to `T2` at 20–160. Bob, on the old state, moves
`P` to 135–275 on `T`. Today's merge compares field by field: it takes `trackId = T2` from Alice and reports a
conflict on both stations, although Alice never touched the platform. Whichever side is chosen, the result is
wrong: Bob's side gives `T2` at 135–275, **100 m off** and still within the track, so no check notices; Alice's
side loses Bob's move.
A prototype that first carries base, mine and theirs through both id logs and then compares gives `T2` at
35–175 without a conflict, in both merge directions.
→ Carry first, then compare (R6); a location is one value (R5).

**F3 — A track is reversed. (tested)** `reverseTrackDirection` flips the switch ports and end marks of the track,
but not its platforms: a platform at 120–260 m on the left of a 400 m track stays at 120–260 on the left —
counted from the other end now, so it has moved 20 m and changed sides.
Every object kind carries its own remap code today, and one of them was forgotten. With signals, balises, masts
and speed boards the number of such places would multiply.
→ One location type, one remap, owned by the core (R4).

**F4 — A track is deleted on which another discipline's objects stand.** Within its own data the layout tool
cascades (end marks and platforms go with the track). Deleting twelve masts of the catenary planner the same way
would destroy work its owner never saw.
→ Never delete another domain's work; orphan it with a finding, and warn before (R7). The warning needs to know
what stands on the track, which only the domains above know → reverse index, derived (R8).

**F5 — A signal hangs on a catenary mast; El signals stand before a neutral section; masts must not block the
sight of a signal.** Signalling and catenary each need the other: the first draft's single direction cannot
hold, and storing references both ways makes a cycle.
All three relations are positions on the same track: same station and side, a distance before a range, a
sight line along the track. They can be **derived** from the shared location, like the Open-Layout-Tool derives
which track ends meet instead of storing it.
→ No stored references between sibling domains; coordination checks derive and check these relations (R9).

**F6 — Station Halle-Trotha.** Station tracks and platforms name their station today by text (`stationName`,
`uicStation`, `code`). The first draft put operational points into the top layer — but tracks and platforms
below it name them, so the references would point upward. Moreover, a station has an identity (name, RL100 code,
UIC number) and an extent, and the extent is set by its entry signals, i.e. by the signalling plan. A new halt
has no RL100 code yet.
→ Identity and extent are two facts with two owners (R10): an operational point register in the core with UUIDs
(codes are attributes, added when they exist), the extent in the signalling domain.

**F7 — Kilometrage of a new line.** The first draft put kilometrage below the net as reference data. For an
existing line it is (DB km lines). For a new line it is defined *by* the designed axis: a start km on a run of
tracks plus km jumps.
→ Designed kilometrage lives next to the tracks in the core, not below them. "Layers" are a picture; the rule
is an acyclic graph of domains (R2).

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

### Cases about storage

**S1 — Editing a platform stores two kilometres of surveyed axis again.** A reference axis keeps a point every
centimetre, at most 2 km: about **1.3 MB of JSON and 20–200 kB gzipped** (measured on synthetic data: smooth vs.
±1.5 mm survey noise). The whole network of the PEK Halle–Könnern data set (22 tracks, 13 switches) is ~15 kB
gzipped. In one domain document, as the first draft had it, every platform edit stores the axis anew — as
oltjson stores the whole record per revision today.
→ Address storage per collection, not per domain; bulk point arrays as blobs of their own (S-R1, S-R2).

**S2 — The optimizer and the browser hash the same data differently. (tested)** Python writes `100.0`, `1e-07`,
`-0.0`; JavaScript writes `100`, `1e-7`, `0`. Content addressing needs identical bytes from both.
→ Canonical JSON after RFC 8785, rounding first, hashing the uncompressed bytes (S-R3).

**S3 — The signalling plan is approved; then a track is shifted by 5 cm.** The signalling document did not
change, so a hash of it alone says "still approved" — yet its meaning changed.
→ An approval pins the domain's hash *and* the hashes of what it depends on; a change below shows it as affected
(S-R6).

**S4 — A timetable viewer and a track editor.** The viewer only needs the core and the operations domain.
The track editor must carry every reference into the net (F1), so it needs every domain that has one.
→ Partial loading is for read-only use; editing the core loads all referencing domains (S-R7).

**S5 — A catenary planner without signalling rights splits a track.** Her check-in changes the signalling domain
(carried references). The server must accept that but nothing more.
→ A changed domain that the author may not write must equal the base carried through the id log, which the
server recomputes (S-R5) — the Open-Layout-Tool already checks measured axes against a right in this way.

**S6 — Twelve construction stages.** Stages are variants (an Open-Layout-Tool decision). With per-collection
hashes, collections no stage changes are stored once. A stage plan (order and dates of the stages) refers to
variants and so must live outside the revisions, at project level, or it would refer to itself.
Open risk: a survey correction of the existing state must reach all twelve stages, which means twelve merges.
F2's rule makes them free of false conflicts, but it is work. This is decided in phase 5.

**S7 — A capacity simulation result.** It follows from the timetable, signalling and core.
→ A cache keyed by the hashes of its inputs, never part of a revision (S-R6).

## 3. Rules for references

| # | Rule | Cases |
|---|---|---|
| R1 | Every collection belongs to one **domain**; a module writes only the domains it owns. The one exception is R4. | — |
| R2 | A domain stores references only into itself or into a declared dependency; the **dependency graph is acyclic**. Inside a domain references may go any way (tracks ↔ switches, as today). | F7 |
| R3 | **References are declared, not guessed.** A schema marks every reference with a core type: `TrackLocation`, `TrackRange`, `TrackEnd`, `TrackPath` or `Ref` (a bare UUID; the schema names its target type). The schema travels with the data (by hash in the manifest). | F1, F10 |
| R4 | **One remap.** Whenever the net changes ids or stations (split, join, reverse, delete, a length change before an object, a plane change), the core carries every declared reference in every domain, in the same step. No module writes its own remap. A build that cannot carry a domain refuses the edit. | F1, F3, F10 |
| R5 | **A location is one value**: compared, merged and carried whole, never `trackId` and `station` separately. | F2 |
| R6 | **Merge: carry first, then compare.** Base, mine and theirs are carried into the newest ids through both id logs before the three-way comparison, so a change that is only carrying is no change. | F2 |
| R7 | **Deleting never deletes another domain's work.** Within its own domain a module cascades. Objects of other domains are left orphaned, with a finding for their owner, and the user is told beforehand what stands there. **An orphan keeps its place in the plane:** in the same step the core turns its location into the anchor `plane` (easting, northing, EPSG, direction as a bearing) without a track, since a station means nothing without its track. Id and every reference to it stay, so it can be attached to a new track later. | F4 |
| R8 | **Upward is derived.** "What stands on this track" is a reverse index built on load from the declared references, never stored. | F4 |
| R9 | **No stored references between sibling domains.** Relations between disciplines are derived from their shared locations and checked by **coordination checks**, which read several domains and store nothing but findings. A relation that cannot be derived from location goes into a coordination domain above both. | F5 |
| R10 | **One fact, one owner.** If two domains seem to need the same fact, it is two facts (design / permitted speed, station identity / extent), or it moves into the core. | F6, F8 |
| R11 | **A location states its anchor**: `station` (follows the track), `km` (keeps its kilometrage) or `plane` (stays where it is). The anchor's value is stored, the others are derived. Objects drawn in a tool default to `station`; existing, imported objects to the form they were delivered in (plane or km). | F9 |

Findings caused by another domain's change (an orphaned signal, a mast now too close) do not block that
change's check-in; they are listed for the owner of the affected domain. A check-in is blocked only by new errors
in domains its author changed beyond carrying.

## 4. Structure

The five layers of the first draft were cut in the wrong places (F6, F7). What remains is a core that everyone
depends on, discipline domains that depend only on the core, and derived results above them:

```
 derived, never in a revision:  coordination checks · simulation results · reverse index   (caches by input hashes)
 ─────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 discipline       layout-work        catenary            signalling             operations
 domains          surveys, reference masts, cantilevers, signals, detection,    train runs,
                  axes, track runs,  wires, sections,    track sections, signal timetable,
                  plan header        El signals          routes, LST switch     rolling stock
                                                         elements, station
                                                         extents, speed profile
                        │                │                   │                     │
                        └────────────────┴─────────┬─────────┴─────────────────────┘
                                                   ▼  stored references only into the core
 core             reference   coordinate systems · km lines and designed kilometrage · operational point register ·
                              catalogue versions (rules, switch forms, profiles)
                  net         tracks (elements, heights, cant, superstructure) · switches and links
                  context     platforms · end marks · structures (bridges, tunnels, level crossings)
                  types       TrackLocation · TrackRange · TrackEnd · TrackPath · Ref
 ─────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 project level    variants · construction stage plan · approvals · comments   (refer into revisions, never back)
```

- **The core is written by the layout tool** (and carried by the core library, R4). Platforms, end marks and
  structures are in the core because several disciplines place things relative to them.
- **One physical thing, several discipline objects.** A turnout is a core switch (geometry, form, topology); the
  signalling domain has its own switch element referring to it (drive, position detection, interlocking name),
  as PlanPro models it. Each discipline names it in its own terms; the core switch carries the geometry once.
- **Words are reserved per meaning.** Today "route" means a named run of tracks (layout), the two ways through a
  turnout, and will mean a signalling route (Fahrstraße). In the core: `TrackPath` for a run of tracks, "route"
  only for signalling.

A located object, here a signal, with its location as one value:

```json
{
  "id": "5d1c…", "kind": "main", "name": "N3",
  "at": { "trackId": "a9f5…", "station": 812.400, "side": "right", "offset": 2.90,
          "direction": "down", "anchor": "station" }
}
```

The signalling schema declares `"at": { "$ref": "core#/$defs/TrackLocation" }`. Any build finds this reference
through the schema, carries it through a split, and can list the signal as standing on track `a9f5…`.

## 5. Storage

### 5.1 Options

| Option | Strengths | Weaknesses |
|---|---|---|
| **A. One JSON document per project** (oltjson today) | simple; diff and merge are pure functions; works offline | every revision stores everything (S1); one schema version for all modules |
| **B. Documents per domain, content-addressed per collection** | unchanged collections cost nothing; per-domain rights, versions and approvals | manifest, hash store and canonical bytes to maintain |
| **C. Relational DB, normalised** | queries across projects, spatial indexes | variants and three-way merge become hard; a second model for offline browsers; tempts to store derived geometry |
| **D. Operation log / CRDT** | live multi-user editing | geometric constructions (splice, turnout insertion) are not field operations; validating a state needs the state anyway |

### 5.2 Recommendation: B, hardened

A project state is logically a set of **domain documents** (unit of ownership, schema and version). Physically it
is a **hash tree**:

```
revision   { parents, author, time, message, manifest: sha256:…, idLog: [ { from, to: [ids] } ] }
   └─ manifest
        ├─ core        { version: "1.0", schema: sha256:…, fields: sha256:…,
        │                collections: { tracks: sha256:…, switches: sha256:…, platforms: sha256:…, … } }
        ├─ layout-work { version: "1.0", schema: sha256:…, dependsOn: ["core"],
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
| S-R5 | **The server checks per domain**: the author's right for every changed domain; a changed domain the author may not write must equal the base carried through the id log. A domain whose own and dependency hashes are unchanged is not validated again. | S5 |
| S-R6 | **Derived results are caches keyed by input hashes** (coordination findings, simulation results, reverse index), never in a revision. An **approval** pins a domain hash plus the hashes of its dependencies. | S3, S7 |
| S-R7 | **Partial loading is for reading.** Editing the core loads every domain that references it. | S4 |
| S-R8 | **Project level, outside revisions**: variants, stage plans, approvals, comments. They refer into revisions; revisions never refer to them. | S6 |
| S-R9 | **Exchange**: a zip container (`.ord`) with the manifest, the canonical collection files, schemas and blobs. A single-file JSON for small projects holds the same canonical documents and gives the same hashes. | — |
| S-R10 | **Versions**: each domain has its own version; the one door runs an explicit, tested migration chain per domain. Behind the door only the current version exists. | F10 |

Server: SQLite (`revision`, `manifest`, `blob`) until several instances or cross-project queries are needed, then
Postgres with the same tables. Spatial search across projects is a derived index, never the source of truth.

## 6. Consequences for the Open-Layout-Tool

What the cases found in today's code, independent of the new modules:

1. **Merge after a split (F2):** a platform moved on a track that the other side split ends up off by the
   distance to the cut (100 m in the example), or loses the move. Fix: carry all three states first, then
   compare; merge a platform's track, stations and side as one value.
2. **Reversal (F3):** platforms are not carried when a track is reversed. Fix: carry platforms like end marks —
   and in the core, one remap for every location.
3. **Names as references (F6):** `stationName`, `uicStation`, `code` become a reference to the operational point
   register when the core is extracted.

## 7. Open questions

1. *Answered:* Rail-Data-Core lives in this repository together with the modules, since R3, R4 and S-R10 need
   all modules on the same core version ([decision 0001](decisions/0001-one-repository.md)).
2. Which module validates the generalised core first — catenary (locations and ranges) or signalling (also
   functional objects)?
3. Should element chains get ids (finer merge, larger records)? oltjson merges a track's elements as one value.
4. Construction stages: twelve variants with merges for every correction of the existing state (S6), or stages
   as overlays on one state? To be decided before phase 5; the storage supports both.
5. Who approves a change to the core schema?
