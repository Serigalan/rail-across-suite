# 0006 — The format rail-across-1.0: one model, versions that count, a door that checks

Status: **accepted** · as of 2026-11-18 · the specification is [rail-across-1.0](../format/rail-across-1.0.md)

## Decision

1. **One file, two parts.** A project is exchanged as **rail-across-1.0** (in its container `.railacross`, 0002): an envelope naming
   the project (`projectId`), the program that wrote it and when (`generator`, `savedAt`), and two parts —
   **infra-data-1.0** with the domain `core` and the layers `civil`, `signalling` and `electrification` (0008), and
   **operations-1.0** with the domains `vehicles` and `timetables` (0012) — and the bulk data both share as blobs,
   each under the hash of its content. A project server keeps the parts apart: the infrastructure in revisions per
   variant, the operation in revisions per project.
2. **One model.** What is kept is what is worked on: an object states the same in the record the tools work on and
   in the document, the document adding only what a file needs (the envelope, the blobs).
3. **Versions are numbers that count.** Every change of a schema raises its domain's version (today `core` 6.0,
   `civil` 3.0, `signalling` 4.0, `electrification` 3.0, `vehicles` 1.0, `timetables` 1.2). A file of an earlier
   version is brought forward number by number at the door. A schema's hash is kept for integrity only, and a file
   carries no schemas.
4. **The door checks shapes, references and content.** Every reference (`x-target` in the schema) names an object
   of its collection; a lifespan's first day is not after its last; what else a schema cannot say. A document of
   the current version that breaks a rule is refused, the path and the id named. A document of before is brought
   forward, and what it names and lacks is cut as deleting that object would have cut it.
5. **What a build does not know, it passes through.** A domain or a member of a later or foreign writer is kept as
   it came and written back. A later version of a domain the reader knows is refused, not misread.
6. **Ids.** Every id is a UUID in lower case; an id of before is replaced by the UUID drawn from it. A track has an
   id, its elements do not: a track's plan (`horizontalAlignment`) and its `heights` are one value each in diff and
   merge. A height
   point has an id only where another depends on it.
7. **Merging and carrying are generic**: over every collection of objects with ids, by what the schemas mark as
   references, member by member — not by a list kept by hand. Two people changing different signals do not
   conflict.
8. **The schemas speak one language**: words instead of numbers for kinds (`straight`/`arc`/`transition`,
   `line`/`station`, `regular`/`opposite`), references ending in `Id` or `Ids`, one statement of the coordinate
   system (EPSG), names made alike. A switch's ports are as OSRD writes a point switch: `A` the tip, `B1` the
   branch, `B2` the main track.
9. **Operational points are objects** with ids of their own. A track, a platform, a signal refers to its point
   (`operationalPointId`); the point's name and codes are its own, stated once.
10. **Km lines are foreign reference data in design elements**, as a track is: in one plane, as runs from km jump
    to km jump, each with its kilometrage at the start (`kmStart`), the way it counts (`counting`) and its plan by
    column (`horizontalAlignment`, as a track's without cant, design speed and details). The elements are taken as
    the source delivers them: the DB's ASCII interface (MDB, record type 24) and the Verm.ESN import state start
    point, bearing, length and radii, and the record keeps them beside the line's points (`design`), fitted at the
    import as a track is. Where a source delivers points only, a run holds them as a blob
    (`[easting, northing, km]`, in the UTM plane of ETRS89 the line's first point lies in) until they are fitted.
11. **A track's plan stands by column** (`horizontalAlignment`, core 6.0), as the OSRD export states it: over N
    elements `stationing`, `coordinates` and `bearingGon` hold N+1 values — each element's start, the last the
    track's end —, the element types, radii and cants at both ends N; the design speed N where an element states
    one; what few elements carry (a switch's route, a role, reasons) by row in `elementDetails`. The names are
    camelCase as everywhere in rail-across; only the OSRD export writes them in snake_case. There is no gap and
    no kink after an arc or a transition: each element is checked from its own start point — its values lead to
    the next point within 1 mm, an arc or a transition ends on the next bearing within what a millimetre of length
    makes of it —, a gap within the millimetre is closed, what lies further apart is refused. Each element is held
    to its own start, not chained from the track's first one: a length a source rounds to the millimetre would
    otherwise carry on from element to element. A document of before is brought forward the same way; a track
    whose elements lie further apart is refused with it.
12. **An import fits a track before it is kept** (the optimizer's `fit`, run by every import dialog). The
    coordinates are what is fixed; bearings and stations are fitted to them. The straights are laid through their
    points; two straights in a row with different bearings become one where the joint between them lies less than
    5 mm off the line through their outer points, else two with a kink. The arcs are then fitted between their
    two straights, tangent to both, keeping their radius — their length changes a little; the transitions keep
    their lengths, also where a transition comes in pieces. A curve group without a straight within it — a
    compound curve, a reverse curve — is fitted as one group, and only where it stays within 1 mm of the points
    it had. A track's first and last point stay exactly where they were; where it begins or ends with a curve,
    its first point and bearing stay, and the curve runs into the line of the next straight. A sliver a cut left,
    shorter than a millimetre, goes. The routes of a switch keep their form, and the form is checked: a switch
    read off the geometry whose radius or end slope lies more than 1 % off its catalogue form is reported. What
    stays as it lay is named in the import report — and refused at the door where it does not hold together.
13. **The project server rewrites what it keeps once** for every change of the format, at its start after the
    update (a data step, `crates/server/src/data_migrations.rs`), so no stored revision is of a version the door
    no longer takes in place.

## Decided, not built yet

- **The km lines in the record as in the document.** The record still keeps a km line as WGS 84 points with a
  kilometrage each, which every tool draws and reads, and carries the design beside it; the tools are to read the
  design itself, so that record and document are one model throughout (point 2). Only a source of points alone —
  the line files shipped with the app (`tiles/km/`) — is then fitted into elements, with the fit the optimizer has
  for axis points; where the fit stays within 1 cm of every point, the elements are kept and the points dropped,
  else the run keeps its points.

## Why

- **Two models with a mapping between them lose.** A status was derived on the way in and derived differently on
  the way out; a station was the same where two texts happened to be equal.
- **A hash tells that two schemas differ, not which is the later.** Every change within a version needed a chain
  of hashes nobody could read.
- **A format that refuses what it does not know** cannot be read by the build before, nor extended by anyone else.
- **Design elements are what the sources state**, exact and about a tenth of a polyline with a point every few
  metres; the same calculation serves tracks and km lines.

## What it costs

- A file of a version the door no longer knows (oltjson-1.1, the envelope before infra-data) does not open; it is
  exported again from a server that migrated it.
- Collections are sets, sorted by id: the order objects were drawn in is not kept.
- Every format change is a step at the door and a data step on the server.
- A track whose elements lie more than a millimetre apart is no longer read. On the project server of 18 November
  2026 that holds Bahnhof Erfurt (831 joints up to 2.9 cm apart), the Machbarkeitsstudie KHB and Wallau–Erndtebrück
  (arcs and transitions ending up to 1.67° off the next element): their revisions stay as they were kept, refused
  until they are imported again and fitted.
