# 0028 — One model, lifespans by day, and a format that counts its versions

Status: **accepted** · 2026-11-12 · replaces the stage register of [0003](0003-construction-stages.md) and
[0027](0027-construction-stages-by-their-days.md) (0027's points 1 and 2, and the stage ids of 5); builds on
[0021](0021-one-format-one-build-the-project-in-the-module.md) and [0025](0025-layers-on-the-core.md)

A review of the code and of the file format, made without regard to what was decided before, found most of the
errors at two places: the register of construction stages, and the mapping between the record the tools work on
and the document that is kept, which loses and invents. This record settles what the format is to be; it is
carried out in phases, each of them released by itself.

## Decision

1. **One model.** What is kept is what is worked on: an object's statement in the record and in the document is
   the same, the document adding only what a file needs (the envelope, the blobs). Every object may carry a
   `lifespan`; an object that is neither newly built nor removed carries none.
2. **No stages, only days.** There is no register of construction stages and nothing refers to one. A lifespan
   states days:

   ```json
   { "lifespan": { "from": "2027-05-01" } }
   { "lifespan": { "from": "2027-05-01", "to": "2031-12-12" } }
   { "lifespan": { "from": "open" } }
   { "lifespan": { "to": "open" } }
   ```

   `from` is the first day the object is in service, `to` the last. The word `"open"` stands where the day is
   not known yet: newly built, removed, the day to be stated. No `from` — it exists today; no `to` — it stays.
   A state of the project (the existing one, any day, the final one) is worked out of the lifespans, never kept.
3. **Operational points are objects** with ids of their own. A track, a platform, a signal refers to its point;
   the point's name and codes are the point's fields, stated once.
4. **The door checks references and content**, not only shapes: every id referred to is there, a lifespan's
   first day is not after its last, and what else a schema cannot say.
5. **Versions are numbers that count.** Every change of a schema raises the domain's version; a file of an
   earlier version is brought forward number by number. The hash of a schema is kept for integrity only — it
   no longer tells versions apart —, and a file no longer carries the schemas.
6. **Merging and the carrying of references are generic**: over every collection of objects with ids, by what
   the schemas mark as references (`x-target`), not by a list kept by hand.
7. **What a build does not know, it passes through.** A domain or a field of a later or foreign writer is kept
   as it came and written back, not refused and not dropped.
8. **The core's schema is cleaned**: one authoritative statement per element, words instead of numbers for
   kinds, one statement of the coordinate system (EPSG), names made alike.
9. **The envelope says where a file comes from**: the program and version that wrote it, when, and the project's
   id. Reference data that is bulk (the km lines) is held as blobs. An operations file names the project, the
   infrastructure revision and the day it was made for.

## Why

- A stage register is a second statement of what the days already say. It had to be kept in step by hand: a
  copied project kept the stage ids of its origin, a lifespan could name a stage that no longer was, and two
  files of the same days differed.
- Two models with a mapping between them lose: a status was derived on the way in and derived differently on
  the way out, a station was the same where two texts happened to be equal.
- A hash tells that two schemas differ, not which is the later; every change within a version needed a chain of
  hashes in the door that nobody could read.
- A format that refuses what it does not know cannot be read by the build before, nor extended by anyone else.

## Consequences

- **Phases**, each with its tests, released by itself:
  A. lifespans by day (points 1 and 2 for the lifespan) — core 3.0, civil 2.0, signalling 3.0,
  electrification 2.0;
  B. operational points as objects (3);
  C. references and content at the door (4);
  D. generic merge and carry (6);
  E. versions by number, the unknown passed through, the envelope (5, 7, 9 in part);
  F. the core's schema cleaned (8);
  G. bulk reference data as blobs, the operations file (9).
- A file of before is read as ever: the door brings it forward. A stage with a day becomes that day — a `from`
  the day itself, a `to` the day before it —, the final stage `"open"`, the existing one no statement. A
  lifespan that named a stage the register lacked is `"open"`.
- The names a stage could carry (`stages[].name`) are not kept: no tool wrote them.
- The record no longer states `status`, `inServiceFrom` and `inServiceTo`; a tool that shows "existing, new,
  removal" reads it off the lifespan. A switch without a lifespan still shows what its tracks say; that it is
  existing against what its tracks say can no longer be stated (a switch kept where its branch goes shows as a
  removal in the plan), what stands at each day is right all the same.
- An open beginning may have an end, a day or an open one: something built for a while whose days are not known
  yet. The door refuses only a day there is none of and a first day after the last.
- The exchange file (OSRD railjson, the suite's extension `olt`) carries the lifespan; a file of before with a
  planning status is read as new or removal with the day open.
- Phase B: the record holds `operationalPoints` and a track or platform its `operationalPoint`; the door copies
  them, the register is no longer drawn from station texts, and a document is unchanged by it. A station a form
  or an import states as text is made the point of that name once, where it enters the record (the
  Layout-Tool's store, `withStationPoints`); a format that names stations by text takes them from the points
  (`withStationTexts`). A platform's `code` was read as its station's RL100 code but used as its number: a
  platform has no number of its own, it is its track's.
- Phase C: the domains go to core 3.1, civil 2.1, signalling 3.1, electrification 2.1 — the schemas' text is the
  same but for the rule they now state, and the meaning changed: every reference (`x-target`) names an object of
  its collection. A document of the current version that names what it lacks is refused, the path and the id
  named. A document of before was free to, the tools reporting it as a finding instead; brought forward, what it
  names and lacks is cut as deleting that object would have cut it — an optional reference alone, an item of a
  list, else the object that requires it, an object the cut leaves short of its schema as a whole, on until
  nothing dangles. The Layout-Tool's store does the same after a write that takes objects away, and only in the
  layers: a switch deleted takes its switch elements along, and what is nothing without them. A tracks file
  holds the tracks alone, without their elements' membership in switches it has not. The project server keeps
  revisions as format 7 and cuts once those of 6 (`0028-references`).
- Phase D: merge and diff find the collections in the records — every list of objects with ids, every
  collection of a carried layer (`signalling.signals`) — and merge each object member by member, a location
  the record states flat as one value; a layer's fields are one value. Two people changing different signals
  no longer conflict on the whole signalling; the civil structures merge object by object and follow a track
  split on the other side. A km line goes by its id (`km-<line number>`, as every source writes it).
- `stagesOf` and `recordAtStage` stay, as what is worked out: the states a project passes through — the
  existing one, every day something changes, the final one — and the record as it stands at one, now the civil
  structures by their lifespans too.
- The project server rewrites its stored revisions once per phase that changes the format, at its start after
  the update.
- 0027's days typed in the track editor's head and its stage connector (`stage_link`) stay as they are.
