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
| `index.js` | `stageOptions(project)` the stages a project offers, `stageMembers(project, key)` the ids of the tracks, switches, platforms and end marks standing at one or on a day, `stageLabel` what a list calls a stage, `dayLabel` a day as the language writes it; for the picker's calendar `monthWeeks`, `monthAfter`, `dayOf`, `isDayKey` |
| `calendar.test.js` | the calendar's months and days |

## How it works

The core is asked with what the stages need of the record alone — ids, statuses, days —, not with the
geometry, and the answer is kept per project object and stage: the store is immutable, so a project that kept
its object did not change.

The map shows every stage at once, as planned, until a point in time is chosen in the picker beside the
working copy's bar (`components/StagePicker.jsx`): a dropdown with a calendar
(`components/stagePicker/DayCalendar.jsx`) that underlines the days something changes on, the existing and
the final state, and those days as shortcuts. Any day stands for the stage in force on it — the core counts
it to the last stage begun on or before it, before every one the existing state; something new or removed
without a day belongs to the final state alone. `map/trackLayers.js` then draws only the tracks, switches,
platforms and end marks standing then, and the structures on those tracks. The point in time chosen is the
session's (`storage.mapStage`), not the record's.

The days are typed in the element table's head — they are the whole track's, not an element's: in the bar
beside the track's name, in a route's table at each of its tracks' headings (`TrackDays` in
`components/trackTable/TrackTableHeader.jsx`, written by `components/TrackTableOverlay.jsx`). A construction stage connector (`stage_link`) joins the track
at A to the one at B1 while that is in service and to the one at B2 from then on; it is made under
Switches → Stage connector (`components/panels/ConnectSwitchPanel/StageLinkForm.jsx`), its roles proposed by
the tracks' days.
