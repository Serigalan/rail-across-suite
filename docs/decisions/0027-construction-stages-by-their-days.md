# 0027 — Construction stages by their days, and a connector for a track rebuilt

Status: **accepted** · 2026-11-12 · builds on [0003](0003-construction-stages.md) (points 2, 3 and 6)

## Decision

1. **A stage between the existing and the final state begins on a day** (`stages[].date`, core 2.0). The tools
   do not edit stages; they state days, and the stages are made of them: the existing state, a stage for every
   day something is put into or taken out of service, in their order, the final state.
2. **A track, switch, platform or end mark states the days it is in service** in the record — `inServiceFrom`
   the first, `inServiceTo` the last — beside its planning status. The door makes the lifespan of them:
   `from` the stage beginning on the first day, `to` the stage beginning the day after the last. A first day
   makes the object new, a last day alone a removal; a status without a day is as before — new from the final
   state, removed at it.
3. **The Layout-Tool's track editor states a track's days**: two columns, *in service from* and *until*, the whole
   track's, whichever of its rows they are typed in. A track rebuilt in part is drawn as parts, each a track
   with its days (0003, point 6).
4. **A construction stage connector** (`stage_link`) joins three track ends at one place without being a switch:
   the track at A to the one at B1 while that is in service, and to the one at B2 from then on — never both. The
   topology joins A to B1 and to B2, never B1 to B2; at a stage, a port whose track is not in service is empty.
   It replaces the two links of 0003, point 6, where the old and the new part meet the staying one at one place.
5. **A record at a stage** (`recordAtStage`, `stagesOf` in the core) is what every tool shows a stage by: the
   Layout-Tool's map, its picker beside the working copy's bar choosing the stage (every one at once, as
   planned, by default); the Construction-Sim (`apps/construction-sim/`), the net stage by stage with what each
   changes; the Kapazitäts-Sim's stages.

## Why

- A planner thinks in dates — "from 2 June 2028" —, not in stage numbers; a stage numbered by hand would have
  to be kept in step with every date it stands for.
- Days make the stages' order: no list to keep, and two tools stating the same day mean the same stage.
- One connector of three ends says what happens at the tie-in in one object: which track stays, which goes,
  which comes — two links say it only together, and a check has to find the pair.

## Consequences

- A stage that no object's day makes any more goes with it; a stage the record keeps (`stages`, with a name)
  stays.
- The days are whole days: an object in service until 1 June and one from 2 June meet without a gap.
- A lifespan that begins at a dated stage and ends at the final one has no record form of both a day and the
  status "removal" without a day; it is stated as "removal" with its first day.
