# stage — the map at a construction stage

`src/core/utils/stage/` · needs the core's `document`

Which construction stages a project has and what stands at one (decision 0027 on 0003): the existing state,
a stage for every day something is put into or taken out of service, the final state. A track, switch,
platform or end mark states the days it is in service — `inServiceFrom` the first, `inServiceTo` the last —
beside its planning status; the core makes the stages of them and says what stands at each
(`stagesOf`, `recordAtStage` of `@rail-across/core/document`, in `crates/doc/src/stages.rs`).

## Files

| File | What it does |
|---|---|
| `index.js` | `stageOptions(project)` the stages a project offers, `stageMembers(project, key)` the ids of the tracks, switches, platforms and end marks standing at one, `stageLabel` what a list calls a stage |

## How it works

The core is asked with what the stages need of the record alone — ids, statuses, days —, not with the
geometry, and the answer is kept per project object and stage: the store is immutable, so a project that kept
its object did not change.

The map shows every stage at once, as planned, until a stage is chosen in the picker beside the working
copy's bar (`components/StagePicker.jsx`); then `map/trackLayers.js` draws only the tracks and switches of
that stage. The stage chosen is the session's (`storage.mapStage`), not the record's.

The days are typed in the element table, two columns for the whole track, whichever of its rows they are
typed in (`components/TrackTableOverlay.jsx`). A construction stage connector (`stage_link`) joins the track
at A to the one at B1 while that is in service and to the one at B2 from then on; it is made under
Switches → Stage connector (`components/panels/ConnectSwitchPanel/StageLinkForm.jsx`), its roles proposed by
the tracks' days.
