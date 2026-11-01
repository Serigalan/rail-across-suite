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
the model, OSRD's example vehicles built in, and the catalogue on the project server — shared by the Fahrzeug-Editor
and the Fahrplan-Sim ([decision 0014](../docs/decisions/0014-vehicles-in-osrds-form.md)).
