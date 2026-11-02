# @rail-across/basemaps — the maps under every module's map

The basemaps of Rail-Across-Suite, shared by the Layout-Tool and the Signalling-Tool, so both lie on the same
maps and their layers panels offer the same: the worldwide OpenStreetMap styles (Liberty, Positron) and
MapTiler's satellite and relief, France's orthophotos (IGN), Germany's basemap.de and DGM5, and the sixteen
states' digital orthophotos, any of them together. Each comes with its preview (`src/thumbnails/`, cuts of the
map itself) and a `labelKey` the module translates.

```js
import { allBasemaps, findBasemap, BASEMAP_GROUPS, DEFAULT_BASEMAP } from '@rail-across/basemaps'
map.setStyle(findBasemap('positron').style)
```

| Export | What it is |
|---|---|
| `allBasemaps(extra)`, `findBasemap(id, extra)` | the basemaps, with a module's own after them (the Layout-Tool's server adds the Länder's DGM1) |
| `BASEMAP_GROUPS`, `DEFAULT_BASEMAP` | the groups of a layers panel in their order; the default (`liberty`) |
| `LANDESVERMESSUNG_STATES`, `lvBasemapId`, `lvStatesOf`, `LANDESVERMESSUNG_THUMBNAIL` | the states' orthophotos: several are one basemap, its id their ids joined by `+` |
| `isElevationBasemap`, `updateElevationRange`, `onElevationRange`, `RELIEF_RAMP`, `fineReliefStyle` | the basemaps coloured by height, their colour scale re-fitted to what is in view, a finer DEM over DGM5 |
| `MAPTILER_KEY` | the key for the satellite and terrain tiles; `VITE_MAPTILER_KEY` sets another at build time |

The previews are bundled by the app that shows them (`new URL(…, import.meta.url)`). The styles name the source
of every tile, as the state surveys' dl-de/by-2-0 asks.

## Tests

`src/basemaps.test.js`: the states as one basemap, a module's own basemaps, the elevation ones.
