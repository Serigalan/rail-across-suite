# @rail-across/pointcloud

Point clouds as plain computation: LAS, LAZ and E57 files read as streams, tiled at several voxel levels, cut
into thin sections, searched for rail heads to trace a track's measured axis, registered onto each other and
checked against the clearance contour. The Layout-Tool runs it in the browser and in a Web Worker, the
project server in its jobs — the same code, which is why it is a package of its own: the server no longer
imports the app.

Nothing here touches a browser API. Where a cloud's tile file lies — the browser's OPFS, the server, a file on
the server's disk — is the caller's business: every function that reads tiles takes the cloud with its `source`,
`{ read(offset, length), readMany(ranges) }`, each giving bytes. LAZ needs the laz-perf module, which the caller
loads (WASM in the browser, `laz-perf` under Node) and passes as `lazPerf`.

```js
import { readCloudHeader, importLevels, scanClearance } from '@rail-across/pointcloud'
```

The package is imported through its index; only tests reach into `src/`. It needs the core
(`@rail-across/core`: geometry, cross sections, tracks, the turnout plane) and `fflate`; plain Node loads it as
it is.

## Files

| Area | Files |
|---|---|
| Reading | `lasHeader.js`, `lasReader.js` (LAS/LAZ chunk by chunk), `lazChunkTable.js` (the LAZ chunk table, ported from LASzip), `e57Reader.js` + `e57Xml.js` (E57), `cloudReader.js` (either) |
| Import | `importPipeline.js` (a file into tiles: one level, or the server's five) |
| Tiles | `tiles.js` (tile format, segments, levels) |
| Planes | `cloudCrs.js` (affine map per 250 m cell instead of proj4 per point), `registration.js` (spatial Helmert from point pairs) |
| Use | `cloudSection.js` (points in a cross section, decoded segments cached), `cloudSlice.js`, `clearanceCheck.js`, `clearanceScan.js`, `railDetect.js`, `railTrace.js` |

What the formats are and why: the Layout-Tool's
[pointCloud](../../apps/layout-tool/docs/modules/pointCloud.md).

## Check

```bash
npm test     # vitest; the LAZ sample is looked for under OLT_LAZ_FIXTURE and skipped where it is not there
```

E57 samples are in `test/fixtures/e57/`, made with libE57Format by `tools/e57fixtures.py`.
