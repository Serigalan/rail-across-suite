# @rail-across/pointcloud

Point clouds as plain computation: LAS, LAZ and E57 files imported and tiled at several voxel levels, the tiles
cut into thin sections, searched for rail heads to trace a track's measured axis, registered onto each other
and checked against the clearance contour. The Layout-Tool runs it in the browser and in its import worker —
which is why it is a package of its own.

The import is Rust: `crates/pointcloud` (the readers — LAZ through the `laz` crate —, the tiles, the levels,
the conversion to the project's plane), built as a WebAssembly module by `crates/pointcloud-wasm` (`npm run
wasm`, into `wasm/`) and called through `src/native.js` (decision 0020, steps 7 and 10). Every call is
synchronous: the file is read through `source.readSync(offset, length)` — a worker's `FileReaderSync` — and
each level's tiles go to `writers[level].append(bytes)`, which answers where it put them. The project server's
jobs run the same crate natively.

```js
import { loadPointCloudModule, importLevels, scanClearance } from '@rail-across/pointcloud'
import moduleUrl from '@rail-across/pointcloud/wasm/rail_across_pointcloud.wasm?url'
await loadPointCloudModule(moduleUrl)
const bodies = importLevels({ source, levels: [1], writers: { 1: writer }, sourceCrs: 5678, targetCrs: 25832 })
```

The rest is JavaScript and touches no browser API. Where a cloud's tile file lies — the browser's OPFS, the
server, memory — is the caller's business: every function that reads tiles takes the cloud with its `source`,
`{ read(offset, length), readMany(ranges) }`, each giving bytes.

The package is imported through its index; only tests reach into `src/`. It needs the core
(`@rail-across/core`: geometry, cross sections, tracks, the turnout plane) and `fflate`.

## Files

| Area | Files |
|---|---|
| Import | `native.js` (the Rust module: header, first points, a file into tiles — one level, or the server's five), `nativeNode.js` (the module from its file, Node) |
| Tiles | `tiles.js` (tile format, segments, levels) |
| Planes | `cloudCrs.js` (affine map per 250 m cell instead of proj4 per point), `registration.js` (spatial Helmert from point pairs) |
| Use | `cloudSection.js` (points in a cross section, decoded segments cached), `cloudSlice.js`, `clearanceCheck.js`, `clearanceScan.js`, `railDetect.js`, `railTrace.js` |

What the formats are and why: the Layout-Tool's
[pointCloud](../../apps/layout-tool/docs/modules/pointCloud.md).

## Rust

`crates/pointcloud` also has the sections, registration, clearance check and scan, rail detection and tracing,
for the project server. `npm run replay pointcloud` (in `packages/core`) records their vectors from this
package's JavaScript; `cargo test -p rail-across-pointcloud` replays them. The readers' and the import's
vectors are frozen since their JavaScript went (step 10); `crates/pointcloud/tests/e57.rs` holds the E57
reader to what libE57Format wrote.

## Check

```bash
npm run wasm   # the import module, which the tests load
npm test       # vitest; the LAZ sample is looked for under OLT_LAZ_FIXTURE and skipped where it is not there
```

E57 samples are in `test/fixtures/e57/`, made with libE57Format by `tools/e57fixtures.py`.
