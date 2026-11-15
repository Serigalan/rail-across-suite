# 0029 — The suite on rail-across-1.0

Status: **accepted** · 2026-11-13 · carries out the specification [rail-across-1.0](../format/rail-across-1.0.md);
builds on [0028](0028-one-model-lifespans-by-day.md), whose points 5 to 9 it completes

The format description `docs/format/rail-across-1.0` is the target of the data model (2026-11-13). Its last
section lists where the implementation differs. This record settles the order in which the suite is brought to
it. Each phase leaves the tests green and is committed by itself; a phase that changes a schema brings the
documents of before forward by a step of the door and, where the project server keeps them, by a data step.

## Decision

1. **The door first, the record after.** The tools work on the record; a phase changes what is kept — the
   document, its schemas and the door's steps — and the conversion between record and document absorbs the
   difference. Where record and document then still differ, the record follows in a phase of its own (6), so
   that no phase has to change every tool at once.
2. **Versions as the specification states them.** The phases 1 and 2 together make core 4.0, civil 3.0 and
   signalling 4.0; phase 3 electrification 3.0; phase 4 vehicles 1.0 and timetables 1.0. A version is in
   progress until its last phase is committed: the project server is not deployed with it before, so that no
   stored revision holds a version that still changes.
3. **The phases:**
   1. *Core 4.0, its words and names:* element, track and side kinds as words (`straight`/`arc`/`transition`,
      `line`/`station`, `regular`/`opposite`), references ending in `Id` (`operationalPointId`, `depId`,
      `detectionPointIds`, `clearSectionIds`, `passOn.leftId`/`rightId`), no plane of an element's own, the
      switch kind `switch` with `B1` the main track and `B2` the branch.
   2. *Ids and km lines:* every id a UUID in lower case, an id of before replaced by the UUID drawn from it
      (`derivedId`) and every reference to it with it — after phase 1 every reference ends in `Id` or `Ids`,
      so the rule needs no list; km lines in one plane — the UTM plane of ETRS89 a line's first point lies in,
      which needs no grid and gives the same numbers on every device — as runs from km jump to km jump with
      `kmStart` and `counting`, a line of before as points held as a blob until it is fitted with elements.
      The project server rewrites what it keeps once (data step `0029-core-4.0`).
   3. *Electrification 3.0:* masts, suspension points, section insulators and contact wires with their voltage.
   4. *Operations as two domains:* vehicles 1.0 with the rolling stock in the object, timetables 1.0 whose train
      points name an operational point or an end mark by id, with the track.
   5. *The file rail-across-1.0:* one `.railacross.json` holding infra-data-1.0 and operations-1.0, the envelope
      naming project, generator and time of saving, no schemas carried, unknown domains and members kept.
   6. *The record follows the document:* the conversion between them is taken out piece by piece, until what
      is kept is what is worked on (0028, point 1).

## Consequences

- Until phase 6 the record keeps its numbers, names, switch ports and km lines in WGS 84; the door converts. A
  record read from a document holds the UUIDs the document gave it; a tool that reads the core beside its own
  layer reads it in the record's words and ids (`recordDomains`), and a cut of dangling references keeps the
  record's ids. A tool sees no change of words before then.
- The specification's section on the implementation is shortened with every phase.
- Phase 6 began with the signalling layer, which the record already kept as the document has it: its references
  are named as the document's (`detectionPointIds`, `clearSectionIds`, `passOn.leftId`/`rightId`); the door still
  takes the names of before on the way in, as a working copy may hold them. Tracks and platforms followed: they name
  their operational point by `operationalPointId`, and state their kind and side as words (`line`/`station`,
  `regular`/`opposite`). An element's kind, `dep`, the switch
  ports, the km lines and the ids of any form follow in pieces of their own.
