# @rail-across/vehicles — the suite's vehicles

The vehicles of Rail-Across-Suite are OSRD's RailJSON rolling stock (`railjson_version` 3.4), kept as they are
([decision 0014](../../docs/decisions/0014-vehicles-in-osrds-form.md)). This package holds what the Fahrzeug-Editor
(`apps/vehicle-editor/`) and the Fahrplan-Sim (`apps/simulation-tool/`) share of them:

```js
import { BUILT_IN, curveFor, fromOsrd, rollingStockOf, useCatalogue } from '@rail-across/vehicles'
```

| Part | What |
|---|---|
| `src/vehicles.js` | the model: OSRD's categories, loading gauges, comforts, traction modes; a new vehicle; `fromOsrd` / `toOsrd` (import and export, OSRD's defaults, per-kilogram Davis coefficients made the whole train's); the curve OSRD would run a mode and comfort with (`curveFor`); what keeps a vehicle from being saved (`problemsOf`); modes, comforts and curves added and taken away; the simulation's rolling stock (`rollingStockOf`); a curve from the starting effort and the power |
| `src/builtIn.js` | OSRD's eleven example vehicles (`data/osrd/`, see [`data/README.md`](data/README.md)), taken into the build with Vite's `import.meta.glob` |
| `src/catalogue.js` | `/api/vehicles` of the project server and `useCatalogue()`: the suite's vehicles and OSRD's, saved with the version they changed (409 on an older one) |

Units are OSRD's: kg, m, m/s, m/s², N, N/(m/s), N/(m/s)²; a curve's speeds in m/s and efforts in N.

## Tests

```sh
npm test
```
