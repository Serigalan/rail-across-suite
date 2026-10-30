# apps

One directory per module with its own build, named after the module: `home/` (the suite's start page: the
sign-in, the user administration, the modules and the projects), `layout-tool/` (the Layout-Tool, which was
the Layout-Tool before it moved here in phase 0), later `signalling-tool/`, `catenary-tool/`,
`timetable-editor/`, `capacity-sim/` (an adapter to OSRD), `construction-sim/`
([decision 0010](../docs/decisions/0010-one-suite-modules-by-task.md)). An app imports packages, never another app
([decision 0001](../docs/decisions/0001-one-repository.md)).
