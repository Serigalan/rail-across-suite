# schematic — the net as a schematic strip

`packages/core/src/schematic/` · needs `geometry`, `track`

The track network as a strip: the lines of the layout become horizontal lanes along one reference axis, every
switch joining two lanes a 45° connection. Only the length along the axis is to scale — read by projecting
every track onto the reference within a corridor — so a switch sits where it sits in reality; which lane a
track takes follows from which side of the reference it lies on. It was the Layout-Tool's (its schematic
overview, `apps/layout-tool/src/core/utils/plan/planSchematicPlan.js`, cuts the strip into sheets) and moved
here so the Signalling-Tool's schematic view lays the net out alike.

## Files

| File | What it does |
|---|---|
| `layout.js` | `schematicLayout`: strands through the switches' through routes, their lanes and x ranges, the switches on them, platforms, the kilometrage along the reference (from the caller's `kmOf`, else the chainage) and `stationX`, the x of a station of a track |
| `strip.js` | `schematicStrip`: the strip for one lane spacing as lines (each naming its tracks), switch triangles, dots, labels and platforms, every item with the planning status it is drawn in; `switchNumber`, `kmMarks`, `kmAt` |
| `place.js` | `schematicPlacer`: where a station of a track lies on the strip, and a stretch of a track between two stations |

## The strip

x is metres along the reference, y whatever unit `laneY` gives: the plan sheet's millimetres, the
Signalling-Tool's screen pixels. The diagonals take the run that 45° asks for at that spacing (`dxOf`), so
the strip is made anew when the scale changes; the layout stays.

## Placing on the strip

A track is the polyline its lines draw: its straight, the step onto another lane, the diagonal of a branch or
a crossover. A station goes onto it by its share of the way from one end of the track to the other, as the
reference measures it — so an object keeps its place along the line where the track runs straight, and stays
on its track where the drawing bends it.

## Tests

`layout.test.js`, `strip.test.js`, `place.test.js`, on the network of `test/schematicFixture.js`.
