import { createReadStream, statSync } from 'node:fs'
import { extname, join, normalize, resolve, sep } from 'node:path'

const TYPES = {
  '.html': 'text/html; charset=utf-8', '.js': 'text/javascript; charset=utf-8', '.mjs': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8', '.json': 'application/json; charset=utf-8', '.svg': 'image/svg+xml',
  '.png': 'image/png', '.jpg': 'image/jpeg', '.webp': 'image/webp', '.ico': 'image/x-icon', '.pdf': 'application/pdf',
  '.wasm': 'application/wasm', '.tif': 'image/tiff', '.tiff': 'image/tiff', '.pmtiles': 'application/octet-stream',
  '.woff2': 'font/woff2', '.pbf': 'application/x-protobuf', '.txt': 'text/plain; charset=utf-8', '.map': 'application/json',
}

function statOr(path) {
  try { return statSync(path) } catch { return null }
}

/**
 * Serve the built suite from one directory — the start page at the root, each
 * module's app under its path — as Caddy does in production, so the API under
 * /api stays on the page's own origin and no CORS is needed. The local package
 * (desktop/, Tauri) starts the server with `--static <site>` and lets its one
 * window load the page from there.
 *
 * GET and HEAD only; Range requests answered (the km line data and the point
 * cloud tiles are read in ranges); directories answer their index.html; a page
 * is asked for again on every load, a hashed asset (/assets/…) may be cached
 * for a year, everything else for an hour. A path that leaves the root, or
 * under /api/, answers 404.
 */
export function createStaticHandler(root) {
  const ROOT = resolve(root)

  /** The file a request names, inside ROOT, or null. */
  const fileOf = (urlPath) => {
    if (urlPath.startsWith('/api/') || urlPath === '/api') return null
    let path
    try { path = decodeURIComponent(urlPath.split('?')[0]) } catch { return null }
    const full = normalize(join(ROOT, path))
    if (full !== ROOT && !full.startsWith(ROOT + sep)) return null
    const stat = statOr(full)
    if (stat?.isDirectory()) return fileOf(join(path, 'index.html'))
    return stat?.isFile() ? { full, size: stat.size } : null
  }

  return async function serveStatic(req, reply) {
    const file = fileOf(req.raw.url ?? '/')
    if (!file) return reply.code(404).send({ error: 'not_found' })
    const asset = file.full.startsWith(join(ROOT, 'assets') + sep) || file.full.includes(`${sep}assets${sep}`)
    const headers = {
      'Content-Type': TYPES[extname(file.full).toLowerCase()] ?? 'application/octet-stream',
      'Accept-Ranges': 'bytes',
      'Cache-Control': file.full.endsWith('.html') ? 'no-cache' : asset ? 'public, max-age=31536000, immutable' : 'public, max-age=3600',
      'X-Content-Type-Options': 'nosniff',
    }
    const range = /^bytes=(\d*)-(\d*)$/.exec(req.headers.range ?? '')
    let start = 0, end = file.size - 1, status = 200
    if (range && (range[1] || range[2])) {
      if (range[1]) { start = Number(range[1]); if (range[2]) end = Math.min(Number(range[2]), end) }
      else start = Math.max(0, file.size - Number(range[2]))
      if (start > end || start >= file.size) {
        return reply.code(416).header('Content-Range', `bytes */${file.size}`).send({ error: 'range_not_satisfiable' })
      }
      status = 206
      headers['Content-Range'] = `bytes ${start}-${end}/${file.size}`
    }
    headers['Content-Length'] = status === 206 ? end - start + 1 : file.size
    reply.hijack()
    reply.raw.writeHead(status, headers)
    if (req.method === 'HEAD') reply.raw.end()
    else createReadStream(file.full, { start, end }).pipe(reply.raw)
  }
}
