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
      `line`/`station`, `regular`/`opposite`), references ending in `Id` (`operationalPointId`, `depId`), no
      plane of an element's own, the switch kind `switch` with `B1` the main track and `B2` the branch; every
      id a UUID in lower case, an id of before replaced by a UUID drawn from it (`derivedId`) and every
      reference to it with it.
   2. *Km lines in design elements:* runs from km jump to km jump, in one plane, with `kmStart` and
      `counting`; a line of before as points, held as a blob, until it is fitted with elements.
   3. *Electrification 3.0:* masts, suspension points, section insulators and contact wires with their voltage.
   4. *Operations as two domains:* vehicles 1.0 with the rolling stock in the object, timetables 1.0 whose train
      points name an operational point or an end mark by id, with the track.
   5. *The file rail-across-1.0:* one `.railacross.json` holding infra-data-1.0 and operations-1.0, the envelope
      naming project, generator and time of saving, no schemas carried, unknown domains and members kept.
   6. *The record follows the document:* the conversion between them is taken out piece by piece, until what
      is kept is what is worked on (0028, point 1).

## Consequences

- Until phase 6 the record keeps its numbers, names and switch ports; the door converts. A tool sees no change
  before then.
- The specification's section on the implementation is shortened with every phase.
