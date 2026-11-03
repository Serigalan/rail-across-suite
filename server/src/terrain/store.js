import { mkdirSync, readdirSync, readFileSync, renameSync, statSync, unlinkSync, utimesSync, writeFileSync } from 'node:fs'
import { homedir } from 'node:os'
import { join } from 'node:path'
import { Tile, readTile } from './tile.js'

const env = (name, fallback) => Number(process.env[name] ?? fallback)
const MEMORY_TILES = env('RA_TERRAIN_MEMORY_TILES', 24)
const DISK_TILES = env('RA_TERRAIN_DISK_TILES', 400)
const FETCH_TIMEOUT = env('RA_TERRAIN_FETCH_TIMEOUT', 90) * 1000
/** A tile no source has is asked for again after this long, not on every point [ms]. */
const MISSING_TTL = 24 * 3600 * 1000
/** Disk cache files: a JSON header line, then the grid as little-endian float32. */
const EXT = '.dgm1'

/** The bytes at `url`, or null where there is no such file. */
export async function download(url) {
  const res = await fetch(url, { headers: { 'User-Agent': 'rail-across terrain' }, signal: AbortSignal.timeout(FETCH_TIMEOUT) })
  if (res.status === 404) return null
  if (!res.ok) throw new Error(`${url}: ${res.status}`)
  return new Uint8Array(await res.arrayBuffer())
}

/**
 * Tiles by key: in memory (the last few used), on disk, or fetched. Two
 * requests wanting the same tile share one download.
 */
export class TileStore {
  constructor({ cacheDir = process.env.RA_TERRAIN_CACHE ?? join(homedir(), '.cache', 'rail-across-terrain'), fetch = download, now = () => Date.now() } = {}) {
    this.cacheDir = cacheDir
    this.fetch = fetch
    this.now = now
    this.memory = new Map()
    this.missing = new Map()
    this.loading = new Map()
  }

  diskPath(key) {
    return join(this.cacheDir, key.replace('/', '_') + EXT)
  }

  fromDisk(key) {
    const path = this.diskPath(key)
    try {
      const data = readFileSync(path)
      const end = data.indexOf(10)
      const head = JSON.parse(data.subarray(0, end).toString('utf8'))
      const body = data.subarray(end + 1)
      const grid = new Float32Array(body.buffer.slice(body.byteOffset, body.byteOffset + body.byteLength))
      if (grid.length !== head.rows * head.cols) return null
      const t = new Date()
      utimesSync(path, t, t)
      return new Tile(grid, head.rows, head.cols, head.x0, head.y0, head.label)
    } catch {
      return null
    }
  }

  toDisk(key, tile) {
    try {
      mkdirSync(this.cacheDir, { recursive: true })
      const head = Buffer.from(JSON.stringify({ rows: tile.rows, cols: tile.cols, x0: tile.x0, y0: tile.y0, label: tile.label }) + '\n')
      const tmp = `${this.diskPath(key)}.tmp`
      writeFileSync(tmp, Buffer.concat([head, Buffer.from(tile.grid.buffer, tile.grid.byteOffset, tile.grid.byteLength)]))
      renameSync(tmp, this.diskPath(key))
      const files = readdirSync(this.cacheDir).filter(n => n.endsWith(EXT))
        .map(n => ({ path: join(this.cacheDir, n), at: statSync(join(this.cacheDir, n)).mtimeMs }))
        .sort((a, b) => a.at - b.at)
      for (const old of files.slice(0, Math.max(0, files.length - DISK_TILES))) unlinkSync(old.path)
    } catch {
      // A cache that cannot be written only costs the next download.
    }
  }

  async load(source, ek, nk) {
    const key = `${source.id}/${ek}_${nk}`
    const cached = this.fromDisk(key)
    if (cached) return cached
    for (const [label, url] of source.vintages) {
      const bytes = await this.fetch(url(ek, nk))
      if (!bytes) continue
      const tile = await readTile(bytes, source.fmt, label ? `${source.id} ${label}` : source.id)
      this.toDisk(key, tile)
      return tile
    }
    return null
  }

  /**
   * { tile, failed }: the tile with its lower-left corner at kilometre (ek,
   * nk) — null where the source has none there — and whether a null is a
   * failed download rather than a tile the source does not have; a map tile
   * drawn without it must not be kept by the browser.
   */
  async get(source, ek, nk) {
    const key = `${source.id}/${ek}_${nk}`
    if (this.memory.has(key)) {
      const tile = this.memory.get(key)
      this.memory.delete(key)
      this.memory.set(key, tile)
      return { tile, failed: false }
    }
    const missingAt = this.missing.get(key)
    if (missingAt != null && this.now() - missingAt < MISSING_TTL) return { tile: null, failed: false }
    if (!this.loading.has(key)) {
      this.loading.set(key, (async () => {
        try {
          const tile = await this.load(source, ek, nk)
          if (tile) {
            this.memory.set(key, tile)
            while (this.memory.size > MEMORY_TILES) this.memory.delete(this.memory.keys().next().value)
          } else {
            this.missing.set(key, this.now())
          }
          return { tile, failed: false }
        } catch {
          // Not kept as missing: the portal may be back.
          return { tile: null, failed: true }
        } finally {
          this.loading.delete(key)
        }
      })())
    }
    return this.loading.get(key)
  }
}
