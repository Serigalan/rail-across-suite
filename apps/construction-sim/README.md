# Construction-Sim

The Baubetriebs-Sim of Rail-Across-Suite ([decision 0027](../../docs/decisions/0027-construction-stages-by-their-days.md);
[roadmap](../../ROADMAP.md), phase 5): a variant's net construction stage by construction stage — the existing
state, every day something is put into or taken out of service, the final state. For each stage the map shows
what is in service then, what comes into service that day (red) and what leaves it after (orange); the list says
it track by track and connector by connector.

Its **construction scenarios** ([decision 0030](../../docs/decisions/0030-conflict-areas-and-construction-scenarios.md))
run a project's timetable on the net of a day: stretches of track closed for a while — picked on the plan, all day or
in a daily window (possession) —, trains taken out, trains diverted by hand on their points. Each train says how it
fares — as planned, diverted around a closure, diverted by hand, not runnable, taken out — beside its regular run:
arrival, delay, running time, waits and conflicts; a train chosen shows both its ways on the plan. The scenarios are
kept with the timetable (timetables 1.2) and saved through the timetable API; the trains run with the simulation
module (`npm run sim` puts it in `public/sim/`).

The net itself it only reads: the days are typed in the Layout-Tool's track editor, the stage connectors made there. The
variant as this browser's working copy holds it, where there is one, else its head. Served under
`/construction/`; the start page's Baubetriebs-Sim lists the projects and opens a variant here:
`?project=<id>&variant=<id>`.
