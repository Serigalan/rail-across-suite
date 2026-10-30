# @rail-across/core — the Rail-Data-Core

The shared model of Rail-Across-Suite in JavaScript: every module reads and writes the railway through it
([data model](../../docs/data-model.md)). It was taken out of the Layout-Tool in phase 1 of the
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
| `signalling` | the catalogue of signal aspects (DB Ril 301 after PlanPro 1.10, Ks with PZB), the content check of the signalling domain, track sections derived over the net | [signalling](docs/signalling.md) |
| `planpro` | reading a PlanPro 1.10 plan: its net as tracks and switches, its signalling as the domain; placing it on another net | [planpro](docs/planpro.md) |
| `document` | the core 1.0 document: domains, schemas, the door, conversion from and to the record, canonical JSON, hashes | [document](docs/document.md) |
| `project` | the project record the app works on (oltjson-1.1), the door to it, settings, validation | [project](docs/project.md) |
| `merge` | diff, three-way merge, carrying ids, carrying every domain's locations across a change of the net (R4) | [merge](docs/merge.md) |

`schemas/` links the JSON Schemas of the domains at the root of the repository ([schemas](../../schemas/README.md)),
and `types/document.d.ts` holds the types derived from them (`node tools/types.mjs`); the Python core
(`packages/core-py`) reads the same schemas and is held to the same vectors (`node tools/vectors.mjs`).
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

Plain Node loads the core as it is, without a bundler or a module hook: a relative import names its file
(`./cant.js`, `../geometry/index.js`), and JSON comes in `with { type: 'json' }`. `tools/modules.mjs` holds the
sources to that.

## Check

```bash
node tools/modules.mjs     # index only, no cycle, nothing outside the package, a page per module, files named
node tools/vectors.mjs     # the shared vectors in schemas/vectors (--check: are they current)
node tools/types.mjs       # types/document.d.ts from the schemas (--check: is it current)
npm test                   # the core's tests (vitest)
```
