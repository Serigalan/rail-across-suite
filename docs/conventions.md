# Conventions

Units, signs and precision that every part of Rail-Across-Suite uses — in data, in code and in its interfaces.
They are taken over from the Open-Layout-Tool (`docs/architecture.md`, *Conventions*, and `docs/data-format.md`,
*Numbers*) and extended by what the [data model](data-model.md) adds. Exchange formats may use other units
(gon, km); they are converted at the edge, never inside.

## Quantities

| Quantity | Unit and sign | Kept to |
|---|---|---|
| Coordinates | `[easting, northing]` in m, in the track's plane (`epsg`) | 1 µm (6 decimals) |
| Lengths, radii, stations | m | 1 µm |
| Stations of height points | m | 1 mm |
| Heights | m, top of the rail that is not raised; vertical datum per track (`heightEpsg`, default DHHN2016) | 0.1 mm (4 decimals) |
| Bearings | degrees from grid north, clockwise | 10⁻⁹° |
| Radii | signed: positive = right-hand curve in the direction of stationing; `null` = straight | 1 µm |
| Curvature (in integrators) | κ = −1/R | — |
| Cant | mm, signed by the raised rail: positive raises the left rail | 0.001 mm |
| Gradients | ‰, positive = rising in the direction of stationing | derived, not stored |
| Speeds | km/h in steps of 5; 0 = unknown, not checked | as entered |
| Kilometrage | m in the data (km 12.345 is `12345`) | as delivered |

Why these precisions: a switch laid into a track read back at 0.1 mm would inherit a tangent break of 10⁻⁵° on a
40 m arc, ten times what the chain checks hold every joint to; heights to 0.1 mm keep a gradient between two close
points right to 0.01 ‰; angles to 10⁻⁹° are below a micrometre over 50 km.

## Tracks and locations

| Item | Convention |
|---|---|
| Station | m from the track's `BEGIN` (the start node of its first element); the running sum of element lengths, never stored on an element |
| Track ends | `BEGIN`, `END` |
| Side | `left` / `right`, seen in the direction of stationing (from `BEGIN` towards `END`) |
| Offset | m, perpendicular from the track axis to the side named, ≥ 0 |
| Direction | `up` = towards `END` (increasing station), `down` = towards `BEGIN`; absent = both |
| Anchor | `station` (follows the track), `km` (keeps its kilometrage), `plane` (keeps its position in the plane) |
| Coordinate system | the project names a system (e.g. `DBREF_GK`, `ETRS89_UTM`), each track its plane as an EPSG code |

## Identity and time

| Item | Convention |
|---|---|
| Ids | UUID, written in lower case; every object that something refers to has one |
| References | the bare id of the target (plus `BEGIN`/`END` for a track end); the schema names the target type; never a name, never an array index |
| Times | ISO 8601 in UTC with `Z`, e.g. `2026-10-29T13:21:07.630Z` |

## Files and numbers

- Numbers are rounded to the precision above **before** they are written. What is derived (polylines, symbols,
  polygons, gradients) is not written at all.
- Content hashes are taken over canonical JSON after RFC 8785, uncompressed; see the data model, S-R3.

## Language

- Code, comments, documentation and commit messages in English; the words of the [glossary](glossary.md).
- Texts the user reads go through translation keys, in German and English.
- Comments explain a behaviour by its reason, not by a decision number.
