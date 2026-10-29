# 0003 — Construction stages as lifespans on one state

Status: **accepted** · replaces the Open-Layout-Tool's decision "time stages are variants, not years on tracks" ·
amended 2026-10-29: who writes the stage list; stages are checked like everything else

## Decision

1. **One record holds the existing state, the final state and everything temporary** — as the Open-Layout-Tool's
   record already holds existing and final state through `status`.
2. **The core holds an ordered list of construction stages** (`stages`): stage 0 is the existing state, the last
   one the final state, each with its period. A project without stages of its own has just these two. The list
   is written by the layout tool, where the parts rebuilt per stage are drawn; the `construction` domain adds
   sequences and closures that refer to the stages. A stage that is removed hands its lifespans on to the stage
   after it.
3. **Every object has a lifespan** `lifespan: { from, to }` — the first stage it exists in, and the first it no
   longer exists in; absent ends are open. The planning status is derived from it:

   | Lifespan | Status | Drawn |
   |---|---|---|
   | none | existing, stays | black |
   | `from` only | new | red |
   | `to` only | removal | yellow |
   | `from` and `to` | temporary | as the stage shows it |

   The lifespan is a core type like the location types, so every domain uses it alike: a temporary track, a
   temporary signal, a temporary catenary.
4. **A stage is a view**: what exists in it. Topology, rule checks, clearance, the check for tracks drawn twice
   and later the capacity run per stage. Every stage is checked as a network, and its errors count like all
   others: only new ones block, so a stage may be incomplete while it is planned.
5. **Objects do not change between stages.** A change is a replacement: the old object ends in stage *k*, the
   new one begins there and names it in `replaces` — as a DB drawing shows old in yellow and new in red.
6. **Rebuilding part of a track** splits it at the tie-in points. The part that stays has no lifespan; the old
   part ends (`to: k`), the new part begins (`from: k`). Each part is joined to the staying part by a
   **connector with two ends** (`link`), and the connector lives as long as the part it joins: one with
   `to: k` to the old part, one with `from: k` to the new.
7. **Containment.** An object exists only while everything it refers to exists: a connector or switch while its
   tracks do, a signal while its track does. What would outlive its target is a finding for its owner, who
   decides — the signal ends with its track, or a new one on the new part `replaces` it. Nothing is changed in
   another domain without its owner (data model, R7).
8. **The existing state stays one continuous track where it was one.** A split lies exactly on the old axis, so
   staying part, connector and old part are the old track; signals on it are carried onto the parts by the core's
   remap. A run of tracks joined only by connectors, with the same line and track number, is shown, named and
   exported as one track — derived, never stored.
9. **Operations per stage** — closed stretches, temporary speed restrictions, which tracks are in operation —
   belong to a `construction` domain that refers into the core.
10. **Variants stay for real alternatives**, e.g. two construction sequences compared.

## Why

| Case | Stages as variants | Stages as lifespans |
|---|---|---|
| A survey correction of the existing state, eight stages planned | eight merges; one forgotten, and the stages differ | once |
| A radius of the final state changed, built from stage 3 on | merged into stages 3–8 | once |
| A temporary track in stages 2–4 | added in 2, merged into 3 and 4, deleted in 5 | one object, `from: 2, to: 5` |
| A signal moved in stage 3 | possible | old one `to: 3`, new one `from: 3`, `replaces` |
| A turnout laid into an existing track in stage 2 | possible | the split holds in every stage; before 2 the parts meet at a connector |
| Two construction sequences compared | variants of variants | two variants of the one record |
| Capacity per stage | possible | per view, cached by hash and stage |

With variants, existing and final state exist once per stage and must be kept equal by hand; with lifespans they
exist once. The Open-Layout-Tool's status is already the case of two stages; this generalises it.

## What it costs

- A change of heights alone (a track lifted in stage 4) is a replacement track with other heights — correct, it
  is construction work, but more effort than editing heights.
- Every check runs once per stage.
- Old and new tracks may overlap in the record; checks that compare tracks (drawn twice, one end held twice) run
  per stage.
- The tools need a stage selector, and the stage a user edits in decides the lifespan of what is drawn.

## Alternatives considered

- **A variant per stage** (the Open-Layout-Tool's decision): every correction of the existing or final state
  becomes one merge per stage.
- **Values per stage on each object** (a radius for stage 2, another for stage 3): every field would become
  stage-dependent, and merge and validation with it. A replacement is what the drawings show anyway.
