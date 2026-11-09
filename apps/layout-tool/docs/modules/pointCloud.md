# pointCloud — point clouds

`src/core/utils/pointCloud/` · needs `km`, `survey`; the computation is
[`@rail-across/pointcloud`](../../../../packages/pointcloud/README.md)

Mobile-mapping and terrestrial scans as spatial constraints for the design: LAS, LAZ and E57 files of several
gigabytes loaded onto the project server and prepared there — on the server, or in the desktop app the server in
its own process; there are no clouds of one device only — cut into thin slices for the cross section, checked
against the clearance contour, searched for rail heads to trace a track's measured axis, and re-referenced onto
each other. The pure functions are the package `@rail-across/pointcloud` (`packages/pointcloud/`): readers and
tiling are its Rust module (`crates/pointcloud-wasm`, decision 0020 of the suite), the same crate the project
server's jobs run; sections, rail detection and trace, registration and clearance are its JavaScript. This module
keeps what only the browser does and re-exports the rest. A function of the package that reads tiles takes each
cloud with its `source`; `cloudReading.js` gives it the server's (`cloudSource.js`).

## Files

| Area | In this module | In `@rail-across/pointcloud` |
|---|---|---|
| Reading a file | `pointCloudWorker.js` + `cloudFile.js` (header and first points of a chosen file, in a worker), `lasText.js` (preview), `cloudProbe.js` (extent and plausibility against the project) | `native.js`: the Rust module (`crates/pointcloud-wasm`) — LAS, LAZ and E57 read synchronously, header and first points |
| Tiles | `cloudCache.js` (OPFS cache of server tiles), `cloudSource.js` (the server's reader of a cloud), `cloudReading.js` (the package's reading functions by project, each cloud with its source), `projectClouds.js` (the clouds of a project, as the server lists them) | `tiles.js` (tile format, segments, levels) |
| Planes | `cloudRegistration.js`, `surveyShift.js` | `cloudCrs.js` (affine map per 250 m cell instead of proj4 per point), `registration.js` (spatial Helmert from point pairs) |
| Use | `cloudPaint.js`, `cloudOutline.js`, `serverRuns.js`, `cloudServer.js`, `cloudFormat.js` | `cloudSection.js` (points in a cross section), `cloudSlice.js`, `clearanceCheck.js`, `clearanceScan.js`, `railDetect.js`, `railTrace.js` |

## Where a cloud lives

Clouds lie on the project server, loaded once ("Punktwolke laden …") and shared with all members of the
project; **nothing about them is in the project record** — 260–400 MB per km do not fit into a revision. In the
desktop app the server is the one in its own process and the clouds lie in its working folder, so the same words
fit both. Viewing is open to all members; changing clouds (loading, delete, re-reference, server runs, saving
measured axes) needs the admin or the right "edit point clouds", checked on the server. The server keeps five
levels and serves tiles by range requests; the browser caches what it fetched in OPFS (packs per server answer,
least recently used dropped past a limit, 2 GB default). The 3D view and re-referencing are `src/server/cloud3d`.

Earlier versions could read a cloud into one browser's OPFS (`pointclouds/<project>/<cloud>/`). Nothing reads
those any more; the start page's local storage dialog lists them so they can be deleted.

## Loading

A delivery is a few long files of several GB. The browser reads only the chosen file's header and, on request,
its first points (the Rust module in a worker, `FileReaderSync`); the file itself goes to the server in resumable
parts, and the server's jobs read it **chunk by chunk**, never whole: LAZ through its chunk table and the `laz`
crate's decompressor, E57 by its pages, XML section and compressed vectors, with each scan's pose applied. The
form asks for the file's plane and height system **every time, without a default** — LAS headers name none, it
differs per delivery, and a default would be clicked through and put the cloud silently beside the track; a
plausibility probe against the project's tracks catches gross mix-ups.

**Tiles** are squares in the file's own plane: mobile-mapping data arrives in scan order, so a tile is written
once the scanner has moved on (a later pass adds a segment). The server makes five levels in one pass: L0
original, L1 2 cm voxel, L2 8 cm, L3 32 cm, L4 1.28 m, with tiles of 2, 2, 8, 32 and 128 m and units of 1, 1, 2,
8, 32 mm so a tile fits in Uint16; colour 8 bit per channel where the file has it. Tile format version 3; versions
1 and 2 stay readable.

- **2 cm voxels** (L1, what the cross section reads by default): one point per voxel, ~260 MB per km instead of
  ~400 at 1 cm; the survey itself is hardly better than 2 cm, and a 10 cm slice still keeps five voxel layers.
  Per point x, y in millimetres from the tile corner and z from the segment's lowest point as Uint16, intensity
  as a byte; sorted by height, then across, then along, delta-coded, zlib — about 3.9 bytes per point.
- **Original** (L0): every point, on the file's own integer grid — a LAS/LAZ comes back bit-exact (rounding to
  absolute millimetres moved the sample's points by up to 0.5 mm, as its offset is not on whole millimetres);
  E57 is kept to 0.1 mm. Three to four times the storage.

Points are converted to the project's plane when read.

**Converting** hundreds of millions of points through proj4 with an NTv2 grid behind it would be slow, so
`cloudCrs.js` uses an **affine map per 250 m cell**: the exact conversion of the cell's centre plus its
Jacobian by central differences; the error against proj4 is far below a millimetre.

## In the cross section

The cross section reads the points within ±5 cm of its plane (a 2D scatter under the drawing; sleepers and
rails turn half transparent so the measured heads show against the planned top of rail). Decoded segments are
cached, so walking the station slider costs only the filtering and the few new tiles. Not Potree: Potree loads
by level of detail and shows only a subset far away, while "does anything reach into the profile" needs the
actual points of the slice.

## Clearance

`clearanceCheck.js` carries the slice's points into the track's frame (over its top of rail, turned back by the
cant about the same running circle) and tests them against the project's clearance contour. A point counts as an
intrusion when it lies inside the outline, **at least 50 mm over the running plane** (rail heads, check rails
and ballast touch the lower edge from inside by survey noise), and outside the permitted intrusion areas (shown
orange, not counted). Depth and nearest distance are measured against the sides and roof, not the lower edge.

Along a whole track (`clearanceScan.js`) the section moves in **0.5 m steps with a 0.5 m slice**, at each
station's gradient and cant, so every point beside the track is looked at once; consecutive steps with intrusions
become one stretch, and a click jumps to its deepest intrusion. Without a gradient a track cannot be checked.

## Rail detection and trace

`railDetect.js` finds the two heads in a slice by their **flangeways**: on the inner side of each head the
surface drops away (the flange runs there), and the two drops face each other at 1435 mm. The outer side of a
head is no help — scanned from the side it smears into ghost points. Each inner flank is measured as the vertical
face 20–35 mm under the head top; the head centre is the flank plus half the nominal head width (49 E5, 54 E4,
60 E2, worn heads included), the axis point the middle of the two centres (see `survey`).

`railTrace.js` walks a guide — a track of the project or a line drawn over the cloud outline — in steps, allows
the track to be up to about a metre off its guide (measured: 0.53–0.84 m between a track and its own Verm.ESN
axis on line 5550; neighbours stand 3.5 m and more apart), tries slice rotations while nothing foresees the
track, and keeps every step's axis point. Turnouts leave gaps. The result is saved as an axis survey or exported
as a point file.

## Long runs on the server

The track-wide clearance scan and the trace run as jobs on the server, with the same JavaScript (over 5 km the
browser would otherwise load 1.5 GB of voxels, 8 GB of originals), when the user may change clouds; a user
without that right runs them in the browser on the tiles read from the server. Results are kept seven days (a trace until saved), at most four
runs per project, two at a time in their own queue beside the preparation queue.

## Re-referencing

A cloud can be fitted onto another with a **spatial Helmert transformation** found by least squares from point
pairs picked in both (`registration.js`):

```
T(p) = c + t + s · R · (p − c)          R = Rz(κ)·Ry(φ)·Rx(ω)
```

with c the centre of the picked points. Always estimated: the shift (east, north, height) and the turn κ about
the vertical; the tilts ω, φ and the scale s only on request — scanned data are metric, and points along a
railway lie nearly in a line and a plane, so the tilts are often poorly determined. A 3D pair gives three
equations, a pair picked in the cross section two (across and height): on a straight the shift along is then
undetermined, on a single arc the turn about its centre — the rank check says so and asks for a 3D pair. Pairs
with a normalised residual over 3.29 (two-sided, α = 0.1 %) are flagged. T is stored on the server with its
pairs, residuals, author and date, outside the revisions, with its own history; the tiles stay unchanged and T
applies when reading. A new T replaces the old one (not chained). Axis surveys remember the T of the clouds they
were traced in and are marked when it changes (`surveyShift.js`).

## Decisions

- **Clouds lie on the project server only** and are shared per project; they are not part of the record.
- **Plane and height system asked on every loading**, no default.
- **2 × 2 m tiles in plane coordinates**, not by station (a station-based store would be stale after every axis
  change and valid for one track only); 2 cm voxels by default, originals bit-exact on request.
- **Scatter in the cross section, not Potree.**
- **Intrusion = inside the outline, ≥ 50 mm over the running plane, outside permitted areas.**
- **Rails found by their flangeways.**
- **Helmert with shift and κ by default**, tilts and scale on request.

## Tests

Here: the survey shift, the preview text, the outline, the paint and the probe. The package has a test file per
reader, the tile format, the import pipeline, the clearance check and scan, rail detection and trace, and
registration; its fixtures are generated by `packages/pointcloud/test/pointCloudFixture.js`, with E57 samples in
`packages/pointcloud/test/fixtures/e57/` (written once with libE57Format, the format's reference implementation, and kept as fixed data).
