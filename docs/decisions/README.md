# Decisions

Each decision that shapes the repository or the shared data has a record here: what was decided, why, and what it
costs. A record states the decision **as it holds now**. When a decision changes, its record is rewritten, and a
decision overruled goes; the way there is in the history of the repository. The records were put in this order on
2026-11-17, from the thirty kept until then.

A record has up to three parts beside its reasons:

- **Decision** — what holds and is built;
- **Decided, not built yet** — what holds and is still to be built;
- **Open** — what is not decided yet, to be taken up again.

| No. | Decision |
|---|---|
| [0001](0001-one-repository-one-licence.md) | One repository, one licence (LGPL-3.0-only), large data outside the history |
| [0002](0002-project-name.md) | The project's name: Rail-Across-Suite |
| [0003](0003-one-suite-one-server.md) | One suite on one server, signing in first, modules named by their task, one build |
| [0004](0004-ui-library.md) | One look for every module, as a library (`@rail-across/ui`) |
| [0005](0005-rust-computes-javascript-shows.md) | What computes is Rust, what shows is JavaScript; the open project lives in the core's module |
| [0006](0006-the-format-rail-across-1.0.md) | The format rail-across-1.0: one model, versions that count, a door that checks |
| [0007](0007-lifespans-by-day.md) | Lifespans by day: one record for every state of the project |
| [0008](0008-core-and-layers.md) | The core and its layers: who writes what |
| [0009](0009-signalling.md) | Signalling after PlanPro and OSRD, its content a catalogue; the stations |
| [0010](0010-planpro-export-only.md) | PlanPro: export only |
| [0011](0011-running-times.md) | Running times with OSRD's envelope simulation, ported to Rust |
| [0012](0012-operations-timetables-and-vehicles.md) | The operation: timetables and vehicles of a project, in revisions |
| [0013](0013-capacity.md) | Capacity: the suite's own simulation, with conflict areas and binding elements |
| [0014](0014-construction-scenarios.md) | Construction scenarios in the Baubetriebs-Sim |

## Decided, not built yet

| Where | What |
|---|---|
| [0003](0003-one-suite-one-server.md) | the plan's title block with the Rail-Across mark |
| [0006](0006-the-format-rail-across-1.0.md) | the km lines in the record as in the document, in design elements |
| [0010](0010-planpro-export-only.md) | PlanPro: crossings and slips, the drawing (`layout.xml`), kilometres along lines |
| [0012](0012-operations-timetables-and-vehicles.md) | vehicles per project; target times; turnarounds from one train into another |
| [0013](0013-capacity.md) | the search for a free train path; studies of many runs as a job on the server |

## Open

| Where | What |
|---|---|
| [0002](0002-project-name.md) | the trademark; a zip container and its media type — before the first public release |
| [0005](0005-rust-computes-javascript-shows.md) | the page's views in the document's terms; the rest of the JavaScript core |
| [0008](0008-core-and-layers.md) | rights per layer; sketches — once several disciplines work apart on one project |
| [0009](0009-signalling.md) | the values of Ril 819.0202 and Ril 483 |
| [0010](0010-planpro-export-only.md) | PlanPro's licence (RailPL); the stated values checked in a PlanPro tool — before the first public release |
| [0011](0011-running-times.md) | what is not ported from OSRD |
| [0012](0012-operations-timetables-and-vehicles.md) | connections; periodic patterns; delay scenarios and stochastic runs |
| [0014](0014-construction-scenarios.md) | speed restrictions at the site, works trains, a diversion's fitness |
