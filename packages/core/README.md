# @rail-across/core — the Rail-Data-Core

The shared model of Rail-Across-Suite in JavaScript: every module reads and writes the railway through it
([data model](../../docs/data-model.md)). It was taken out of the Open-Layout-Tool in phase 1 of the
[roadmap](../../ROADMAP.md), without a change in behaviour; the app now imports it like any other package.

| Module | Subject | Page |
|---|---|---|
| `geometry` | planes and datums, elements, transitions, parallels, plane changes, chain invariants | [geometry](docs/geometry.md) |
| `rules` | rule catalogues (DB Ril 800.0110), expression language, alignment checks, cant/speed/length limits | [rules](docs/rules.md) |
| `track` | tracks, switch records and ports, heights, links, track ends, platforms, routes | [track](docs/track.md) |
| `crossSection` | cross section, superstructure, clearance contours | [crossSection](docs/crossSection.md) |
| `topology` | the connection diagram | [topology](docs/topology.md) |
| `switch` | switch forms, routes, symbols, crossings, switch connections, deletion | [switch](docs/switch.md) |
| `commands` | what an edit commits: new tracks and switches, connecting, splicing, reconnecting | [commands](docs/commands.md) |
| `survey` | measured axes, reference axes, shift values, alignment fit | [survey](docs/survey.md) |
| `project` | the project record: format oltjson-1.1, settings, validation | [project](docs/project.md) |
| `merge` | diff, three-way merge, carrying ids | [merge](docs/merge.md) |

`constraints/` holds the rule and form catalogues as data, shared with the Python optimizer
(`services/optimizer/olt_optimizer/constraints` links here); `test/` holds helpers and fixtures the tests of the
core and of the apps share.

## Use

```js
import { validateProject, parseProjectsPayload } from '@rail-across/core/project'
import { mergeProject } from '@rail-across/core/merge'
```

A module is imported through its index, `@rail-across/core/<module>`, never a file of it; only tests reach inside
(`@rail-across/core/src/<module>/<file>.js`). The core imports nothing of an app (`npm run check` at the root)
and nothing outside its dependencies.

The sources import each other without file extensions, as the bundler resolves them. Under plain Node, the
project server loads them through its module hook (`server/src/esmHook.mjs`).

## Check

```bash
node tools/modules.mjs     # index only, no cycle, nothing outside the package, a page per module
npm test                   # the core's tests (vitest)
```
