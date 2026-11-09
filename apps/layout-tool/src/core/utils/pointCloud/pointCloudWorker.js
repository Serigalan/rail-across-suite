// The point cloud import, off the main thread: a file of gigabytes takes
// minutes, and the page stays usable meanwhile. The work is the Rust module's
// (crates/pointcloud-wasm, decision 0020): it reads the file and writes the
// tiles synchronously — the file through FileReaderSync, the tiles through a
// synchronous access handle, the fast OPFS path and one only a worker may
// hold. Messages in:
//   { type: 'import', file, projectId, cloudId, meta, sourceCrs, targetCrs, original, base }
// and out: { type: 'progress', … }, { type: 'done', index }, { type: 'error', message }.
//   { type: 'header', file } answers { type: 'header', header }
//   { type: 'preview', file, count } answers { type: 'preview', header, points }
// with the file's first points, read without importing anything (lasText).
//
// The module runs until it is done, deaf to messages meanwhile: an import is
// stopped by ending the worker, and the page clears away what it had written
// (pointCloudImport). One that fails removes its directory here.
import moduleUrl from '@rail-across/pointcloud/wasm/rail_across_pointcloud.wasm?url'
import { importLevels, loadPointCloudGrid, loadPointCloudModule, readCloudHeader, readFirstPoints } from '@rail-across/pointcloud'
import { projectDir, INDEX_FILE, TILES_FILE } from './cloudStore'
import { loadNtv2Grid, loadGridsFor, loadedGrids, crsDatum, utmToWgs84 } from '@rail-across/core/geometry'

/** The file, read synchronously as the module asks for it. */
function fileSource(file) {
  const reader = new self.FileReaderSync()
  return {
    size: file.size,
    readSync: (offset, length) => new Uint8Array(reader.readAsArrayBuffer(file.slice(offset, offset + length))),
  }
}

/** The grids the conversion runs on, handed to the module once loaded here. */
const handed = new Set()
function handGrids() {
  for (const grid of loadedGrids()) {
    if (handed.has(grid.key)) continue
    loadPointCloudGrid(grid)
    handed.add(grid.key)
  }
}

async function runImport({ file, projectId, cloudId, meta, sourceCrs, targetCrs: wanted, original = false, base }) {
  // The original resolution stays in the file's plane (Entscheidung 145).
  const targetCrs = original ? sourceCrs : wanted
  const dir = await projectDir(projectId, true)
  const cloudDir = await dir.getDirectoryHandle(cloudId, { create: true })
  let tiles = null
  try {
    await loadPointCloudModule(new URL(moduleUrl, base))
    const source = fileSource(file)
    const header = readCloudHeader(source)
    // The grids the conversion runs on — the same the page loaded at start,
    // and the regional one the cloud's area may have.
    if (Number(sourceCrs) !== Number(targetCrs)) {
      await loadNtv2Grid({ base })
      const datums = [crsDatum(sourceCrs), crsDatum(targetCrs)].filter(Boolean)
      if (datums.length) {
        const [w, s] = utmToWgs84(header.min[0], header.min[1], sourceCrs)
        const [e, n] = utmToWgs84(header.max[0], header.max[1], sourceCrs)
        await loadGridsFor([Math.min(w, e), Math.min(s, n), Math.max(w, e), Math.max(s, n)], datums, { base })
      }
      handGrids()
    }

    tiles = await (await cloudDir.getFileHandle(TILES_FILE, { create: true })).createSyncAccessHandle()
    let at = 0
    const writer = {
      append(bytes) {
        const offset = at
        tiles.write(bytes, { at })
        at += bytes.length
        return offset
      },
    }
    const level = original ? 0 : 1
    const body = importLevels({
      source, levels: [level], writers: { [level]: writer }, sourceCrs, targetCrs,
      onProgress: (p) => self.postMessage({ type: 'progress', ...p }),
    })[level]
    tiles.flush()
    tiles.close()
    tiles = null

    const index = {
      ...meta, ...body,
      crs: Number(targetCrs), sourceCrs: Number(sourceCrs),
      file: header.format === 'e57'
        ? { name: file.name, size: file.size, format: 'e57', e57Version: header.version, scans: header.scans.length }
        : { name: file.name, size: file.size, lasVersion: header.version, pointFormat: header.pointFormat },
      createdAt: new Date().toISOString(),
    }
    const indexHandle = await (await cloudDir.getFileHandle(INDEX_FILE, { create: true })).createSyncAccessHandle()
    const json = new TextEncoder().encode(JSON.stringify(index))
    indexHandle.truncate(0)
    indexHandle.write(json, { at: 0 })
    indexHandle.flush()
    indexHandle.close()
    self.postMessage({ type: 'done', index: { ...index, id: cloudId } })
  } catch (err) {
    try { tiles?.close() } catch { /* already closed */ }
    await dir.removeEntry(cloudId, { recursive: true }).catch((e) => console.error('pointcloud cleanup failed', e))
    self.postMessage({ type: 'error', message: String(err?.message ?? err) })
  }
}

/** The header of a file, or its first points too — read without importing anything. */
async function runRead({ type, file, count, base }) {
  try {
    await loadPointCloudModule(new URL(moduleUrl, base))
    const source = fileSource(file)
    const header = readCloudHeader(source)
    if (type === 'header') self.postMessage({ type, header })
    else self.postMessage({ type, header, points: readFirstPoints(source, count) })
  } catch (err) {
    self.postMessage({ type: 'error', message: String(err?.message ?? err) })
  }
}

self.onmessage = ({ data }) => {
  if (data?.type === 'import') runImport(data)
  else if (data?.type === 'header' || data?.type === 'preview') runRead(data)
}
