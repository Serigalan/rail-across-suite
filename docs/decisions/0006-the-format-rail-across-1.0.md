# 0006 — The format rail-across-1.0: one model, versions that count, a door that checks

Status: **accepted** · as of 2026-11-17 · the specification is [rail-across-1.0](../format/rail-across-1.0.md)

## Decision

1. **One file, two parts.** A project is exchanged as **rail-across-1.0** (in its container `.railacross`, 0002): an envelope naming
   the project (`projectId`), the program that wrote it and when (`generator`, `savedAt`), and two parts —
   **infra-data-1.0** with the domain `core` and the layers `civil`, `signalling` and `electrification` (0008), and
   **operations-1.0** with the domains `vehicles` and `timetables` (0012) — and the bulk data both share as blobs,
   each under the hash of its content. A project server keeps the parts apart: the infrastructure in revisions per
   variant, the operation in revisions per project.
2. **One model.** What is kept is what is worked on: an object states the same in the record the tools work on and
   in the document, the document adding only what a file needs (the envelope, the blobs).
3. **Versions are numbers that count.** Every change of a schema raises its domain's version (today `core` 5.0,
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
   id, its elements do not: a track's `elements` and its `heights` are one value each in diff and merge. A height
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
10. **Km lines are foreign reference data in design elements**, as a track is: in one plane (the UTM plane of
    ETRS89 a line's first point lies in), as runs from km jump to km jump, each with its kilometrage at the start
    (`kmStart`) and the way it counts (`counting`). Where a source delivers points only, a run holds them as a blob
    (`[easting, northing, km]`) until they are fitted.
11. **The project server rewrites what it keeps once** for every change of the format, at its start after the
    update (a data step, `crates/server/src/data_migrations.rs`), so no stored revision is of a version the door
    no longer takes in place.

## Decided, not built yet

- **The km lines in the record as in the document.** The record holds a km line as the format does — one plane,
  runs, design elements —, no longer as WGS 84 points with a kilometrage each. The elements are taken as the source
  delivers them: the DB's ASCII interface (MDB, record type 24) and the VermESN import state start point, bearing,
  length and radii, which are kept instead of being turned into points. Only a source of points alone — the line
  files shipped with the app (`tiles/km/`) — is fitted into elements, with the fit the optimizer has for axis
  points; where the fit stays within 1 cm of every point, the elements are kept and the points dropped, else the
  run keeps its points. With that, record and document are one model throughout (point 2).

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
