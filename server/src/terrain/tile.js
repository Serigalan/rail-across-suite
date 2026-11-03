import { unzipSync } from 'fflate'
import { fromArrayBuffer } from 'geotiff'

/** Heights below this are the files' "no data", not ground. */
const NO_DATA_BELOW = -1000

/**
 * A 1 m grid of heights: row 0 the northernmost, (x0, y0) the centre of its
 * first cell, NaN where there is no data.
 */
export class Tile {
  constructor(grid, rows, cols, x0, y0, label) {
    Object.assign(this, { grid, rows, cols, x0, y0, label })
  }

  /** Bilinear between the four nearest cells, clamped at the tile edge; null where a cell with a say has no data. */
  height(e, n) {
    const { grid, rows, cols } = this
    const c = Math.min(Math.max(e - this.x0, 0), cols - 1)
    const r = Math.min(Math.max(this.y0 - n, 0), rows - 1)
    const c0 = Math.floor(c)
    const r0 = Math.floor(r)
    const c1 = Math.min(c0 + 1, cols - 1)
    const r1 = Math.min(r0 + 1, rows - 1)
    const tc = c - c0
    const tr = r - r0
    let sum = 0
    for (const [v, w] of [
      [grid[r0 * cols + c0], (1 - tc) * (1 - tr)], [grid[r0 * cols + c1], tc * (1 - tr)],
      [grid[r1 * cols + c0], (1 - tc) * tr], [grid[r1 * cols + c1], tc * tr],
    ]) {
      if (w <= 0) continue
      if (!Number.isFinite(v)) return null
      sum += v * w
    }
    return sum
  }
}

/**
 * An XYZ list as a Tile. The grid is placed by the coordinates in the file,
 * not by the order of its lines or a fixed half-metre offset: the sources
 * differ in both (Thüringen's 2014–2019 puts its cells on whole metres, the
 * rest on half metres; Berlin lists from the south, Thüringen from the north).
 * Several lists — the four kilometres of a Baden-Württemberg tile — are one
 * grid once joined.
 */
export function parseXyz(text, label) {
  const values = text.trim().split(/\s+/).map(Number)
  if (values.length < 3 || values.length % 3 || values.some(v => Number.isNaN(v))) throw new Error('not an XYZ list')
  let x0 = Infinity
  let y0 = -Infinity
  for (let i = 0; i < values.length; i += 3) {
    x0 = Math.min(x0, values[i])
    y0 = Math.max(y0, values[i + 1])
  }
  let rows = 0
  let cols = 0
  for (let i = 0; i < values.length; i += 3) {
    cols = Math.max(cols, Math.round(values[i] - x0) + 1)
    rows = Math.max(rows, Math.round(y0 - values[i + 1]) + 1)
  }
  const grid = new Float32Array(rows * cols).fill(NaN)
  for (let i = 0; i < values.length; i += 3) {
    const z = values[i + 2]
    grid[Math.round(y0 - values[i + 1]) * cols + Math.round(values[i] - x0)] = z < NO_DATA_BELOW ? NaN : z
  }
  return new Tile(grid, rows, cols, x0, y0, label)
}

/**
 * A single-band GeoTIFF as a Tile, placed by its tie point and pixel size. A
 * tie point names the corner of the first pixel (PixelIsArea, the GeoTIFF
 * default) or its centre (PixelIsPoint); the grid is kept by cell centres.
 */
export async function parseTif(bytes, label) {
  const tiff = await fromArrayBuffer(bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength))
  const image = await tiff.getImage()
  if (image.getSamplesPerPixel() !== 1) throw new Error('not a single band of heights')
  const dir = image.getFileDirectory()
  const tie = dir.ModelTiepoint
  const scale = dir.ModelPixelScale
  if (!tie || !scale) throw new Error('not georeferenced')
  const [sx, sy] = [scale[0], scale[1]]
  if (Math.abs(sx - 1) > 1e-6 || Math.abs(sy - 1) > 1e-6) throw new Error('not a 1 m grid')
  const half = image.getGeoKeys()?.GTRasterTypeGeoKey === 2 ? 0 : 0.5
  const x0 = tie[3] - tie[0] * sx + half * sx
  const y0 = tie[4] + tie[1] * sy - half * sy
  const [band] = await image.readRasters()
  const grid = Float32Array.from(band)
  const noData = image.getGDALNoData()
  for (let i = 0; i < grid.length; i++) {
    if ((noData != null && grid[i] === Math.fround(noData)) || grid[i] < NO_DATA_BELOW) grid[i] = NaN
  }
  return new Tile(grid, image.getHeight(), image.getWidth(), x0, y0, label)
}

/** The bytes of one download as a Tile, by the source's format. */
export async function readTile(bytes, fmt, label) {
  if (fmt === 'tif') return parseTif(bytes, label)
  const files = unzipSync(bytes)
  const names = Object.keys(files)
  if (fmt === 'tif-zip') {
    const name = names.find(n => /\.tiff?$/i.test(n))
    if (!name) throw new Error('no GeoTIFF in the archive')
    return parseTif(files[name], label)
  }
  const lists = names.filter(n => /\.xyz$/i.test(n))
  if (!lists.length) throw new Error('no XYZ in the archive')
  return parseXyz(lists.map(n => new TextDecoder().decode(files[n])).join('\n'), label)
}
