# apps

One directory per module with its own build, named after the app: `home/` (the suite's start page, the
modules and the projects), `open-layout-tool/` (the Open-Layout-Tool, moved here in phase 0), later `open-signalling-tool/`, `open-catenary-tool/`, `open-timetable-tool/`,
`open-capacity-sim/` (an adapter to OSRD), `open-construction-sim/`. An app imports packages, never another app
([decision 0001](../docs/decisions/0001-one-repository.md)).
