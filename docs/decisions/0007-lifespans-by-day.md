# 0007 — Lifespans by day: one record for every state of the project

Status: **accepted** · as of 2026-11-17

## Decision

1. **One record holds the existing state, the final state and everything temporary.** Variants stay for real
   alternatives, two construction sequences compared, say.
2. **Every object may carry a lifespan, and a lifespan states days:**

   ```json
   { "lifespan": { "from": "2027-05-01" } }
   { "lifespan": { "from": "2027-05-01", "to": "2031-12-12" } }
   { "lifespan": { "from": "open" } }
   { "lifespan": { "to": "open" } }
   ```

   `from` is the first day the object is in service, `to` the last; the days are whole days. The word `"open"`
   stands where the day is not known yet: newly built, removed, the day to be stated. No `from` — it exists today;
   no `to` — it stays. An object that is neither newly built nor removed carries none. The planning status a plan
   shows — existing, new, removal, temporary — is read off the lifespan.
3. **There is no register of construction stages.** A state of the project is worked out of the lifespans, never
   kept (`stagesOf`, `recordAtStage` in the core): the existing state, every day something is put into or taken
   out of service, the final state. Every tool shows a state that way — the Layout-Tool's picker beside the
   working copy's bar (any day stands for the state in force on it), the Baubetriebs-Sim, the Kapazitäts-Sim.
4. **Objects do not change between states.** A change is a replacement: the old object ends, the new one begins.
5. **A track rebuilt in part is drawn as parts**, each a track with its days, split exactly on the old axis, so
   the existing state stays one continuous track where it was one. Where the old and the new part meet the staying
   one at one place, a **construction stage connector** (`stage_link`) joins the three ends without being a switch:
   the track at A to the one at B1 while that is in service, to the one at B2 from then on — never both.
6. **The Layout-Tool's track editor states a track's days**, *in service from* and *until*, for the whole track:
   in the table's head, not in a column of its elements.
7. **An object exists only while what it refers to exists** — a connector or switch while its tracks do, a signal
   while its track does. What would outlive its target is a finding for its owner, who decides; nothing is changed
   in another layer without its owner.
8. **Every state is checked as a network**, and its errors count like all others: only new ones block, so a state
   may be incomplete while it is planned.

## Why

- **A planner thinks in dates** — "from 2 June 2028" —, not in stage numbers.
- **A stage register is a second statement of what the days already say.** It had to be kept in step by hand: a
  copied project kept the stage ids of its origin, a lifespan could name a stage that no longer was, and two files
  of the same days differed.
- **One record, not a variant per stage.** With variants, the existing and the final state exist once per stage and
  must be kept equal by hand: a survey correction with eight stages planned is eight merges.
- **One connector of three ends** says what happens at a tie-in in one object; two links say it only together.

## What it costs

- A change of heights alone (a track lifted from a day on) is a replacement track with other heights — correct,
  it is construction work, but more effort than editing heights.
- Every check runs once per state, and old and new tracks may overlap in the record.
- A switch without a lifespan shows what its tracks say; that it is existing against what its tracks say cannot be
  stated.
