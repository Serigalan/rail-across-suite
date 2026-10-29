# apps

One directory per module with its own build: `layout-tool/` (the Open-Layout-Tool, moved here in phase 0), later
`catenary-tool/`, `signalling-tool/`, `timetable-tool/`, … An app imports packages, never another app
([decision 0001](../docs/decisions/0001-one-repository.md)).
