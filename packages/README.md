# packages

Shared code: [`core/`](core/README.md) (the Rail-Data-Core in JavaScript, `@rail-across/core`) and, to come,
`core-py/` (the same core for Python), extracted from the Layout-Tool in phase 1 of the
[roadmap](../ROADMAP.md).

[`ui/`](ui/README.md) (`@rail-across/ui`) is the look of the suite as a library: colours, buttons, fields, dialogs,
lists and the shell of a tool, for every module's pages ([decision 0012](../docs/decisions/0012-ui-library.md)).

[`basemaps/`](basemaps/README.md) (`@rail-across/basemaps`) are the maps every module's map can lie on, shared by the
Layout-Tool and the Signalling-Tool.

[`working-copy/`](working-copy/README.md) (`@rail-across/working-copy`) is the local working copy of a variant in the
browser, one per variant, shared by the Layout-Tool, the Signalling-Tool and the suite's start page.

[`vehicles/`](vehicles/README.md) (`@rail-across/vehicles`) are the suite's vehicles as OSRD's RailJSON rolling stock:
the model, OSRD's example vehicles built in, and the catalogue on the project server — shared by the Fahrzeug-Editor,
the Fahrplan-Sim and the Fahrplan-Editor ([decision 0014](../docs/decisions/0014-vehicles-in-osrds-form.md)).

[`running-times/`](running-times/README.md) (`@rail-across/running-times`) is a train's way over a variant's net, what
the physics needs of it, the stations along it, and the simulation module (OSRD's simulation core in Rust, `sim/`) —
shared by the Fahrplan-Sim and the Fahrplan-Editor ([decision 0013](../docs/decisions/0013-running-times-in-rust.md)).

[`timetables/`](timetables/README.md) (`@rail-across/timetables`) are a project's timetables: their trains, the runs
of those over a variant's net, and the timetables on the project server
([decision 0016](../docs/decisions/0016-timetables-of-a-project.md)).
