# Construction-Sim

The Baubetriebs-Sim of Rail-Across-Suite ([decision 0027](../../docs/decisions/0027-construction-stages-by-their-days.md);
[roadmap](../../ROADMAP.md), phase 5): a variant's net construction stage by construction stage — the existing
state, every day something is put into or taken out of service, the final state. For each stage the map shows
what is in service then, what comes into service that day (red) and what leaves it after (orange); the list says
it track by track and connector by connector.

It only reads: the days are typed in the Layout-Tool's track editor, the stage connectors made there. The
variant as this browser's working copy holds it, where there is one, else its head. Served under
`/construction/`; the start page's Baubetriebs-Sim lists the projects and opens a variant here:
`?project=<id>&variant=<id>`.
