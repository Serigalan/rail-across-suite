// Reading a chosen point cloud file off the main thread, before it is loaded
// onto the server: the Rust module (crates/pointcloud-wasm, decision 0005)
// reads the file synchronously, through FileReaderSync, which only a worker
// has. Messages in and out:
//   { type: 'header', file } answers { type: 'header', header }
//   { type: 'preview', file, count } answers { type: 'preview', header, points }
// with the file's first points (lasText); { type: 'error', message } when the
// file cannot be read.
import moduleUrl from '@rail-across/pointcloud/wasm/rail_across_pointcloud.wasm?url'
import { loadPointCloudModule, readCloudHeader, readFirstPoints } from '@rail-across/pointcloud'

/** The file, read synchronously as the module asks for it. */
function fileSource(file) {
  const reader = new self.FileReaderSync()
  return {
    size: file.size,
    readSync: (offset, length) => new Uint8Array(reader.readAsArrayBuffer(file.slice(offset, offset + length))),
  }
}

/** The header of a file, or its first points too. */
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
  if (data?.type === 'header' || data?.type === 'preview') runRead(data)
}
