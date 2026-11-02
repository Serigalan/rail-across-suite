# Layout-Tool — documentation

The Layout-Tool designs railway track layouts on a map: alignments of straights, arcs and transitions, turnouts
and crossings after DB Ril 800.0120, vertical alignments, platforms, cross sections with clearance and point
clouds, rule checks after DB Ril 800.0110, plans, and exchange with OSRD, Verm.ESN, ProVI and the DB inventory. It
runs in the browser; a project server adds users, variants and revisions, a Python service optimization and
conversions.

These pages describe what each part does, how it computes and why it is built the way it is. They are written for
whoever maintains or reviews the tool. User guidance is in the app (*Leitfaden*, `public/guideline*.html`).

## Overview

- [Architecture](architecture.md) — layers, builds, modules and their order, data flow, conventions, testing
- [Data format](data-format.md) — oltjson-1.1, field by field, and why it is shaped this way
- [Interface layer](interface.md) — store, shell, panels, overlays, map, translations
- [Server parts](server.md) — browser server parts, the project server, the Python service

## Modules of `src/core/utils`

| Module | |
|---|---|
| [geometry](../../../packages/core/docs/geometry.md) | coordinate systems, NTv2 grids, elements, clothoid and Bloss transitions, parallels, plane changes |
| [track](../../../packages/core/docs/track.md) | tracks, switch records and ports, heights, links, track ends, platforms, routes |
| [switch](../../../packages/core/docs/switch.md) | switch forms, turnouts on any stem, crossings, switch connections, connecting to a track, deletion |
| [rules](../../../packages/core/docs/rules.md) | the rule catalogue and its expression language, cant, speed and length limits, alignment checks |
| [elevation](modules/elevation.md) | Höhenplan rules, the turnout plane and coupling points, a switch's other tracks on request, terrain |
| [crossSection](../../../packages/core/docs/crossSection.md) | cross section, superstructure, clearance contours |
| [commands](../../../packages/core/docs/commands.md) | what the dialogs commit |
| [splice](modules/splice.md) | joining two track ends |
| [survey](../../../packages/core/docs/survey.md) | measured axes, reference axes, shift values, alignment fit |
| [exchange](modules/exchange.md) | RailJSON/OSRD, Verm.ESN, ProVI, Gleislage CSV, MDB |
| [plan](modules/plan.md) | plan export |
| [topology](../../../packages/core/docs/topology.md) | connection diagram |
| [km](modules/km.md) | kilometrage lines |
| [project](../../../packages/core/docs/project.md) | the record as a whole: format, settings, validation |
| [merge](../../../packages/core/docs/merge.md) | three-way merge and comparison |
| [pointCloud](modules/pointCloud.md) | point clouds |
| [common](modules/common.md) | small helpers |

## Elsewhere

- [`README.md`](../README.md) — features, getting started, the local version
- [`deploy/README.md`](../../../deploy/README.md) — operating the suite's server
- [`server/README.md`](../../../server/README.md) — the project server
- [`services/optimizer/README.md`](../../../services/optimizer/README.md) — the optimizer (German)
- [`tools/local/README.md`](../tools/local/README.md) — the local version as handed out
- `packages/core/constraints/*.json` — the rulebooks themselves (DB Ril 800.0110, 800.0120, 800.0130, physics)
