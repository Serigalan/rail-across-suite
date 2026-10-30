import { sha256OfText } from './sha256.js'

// Canonical JSON after RFC 8785 (JCS): the one byte form of a value, so that
// the browser, the server and the Python core hash the same data alike (data
// model S-R3). Object members sorted by their names' UTF-16 code units,
// numbers as ECMAScript writes them (`100`, `1e-7`, `-0` as `0`), strings
// escaped as JSON.stringify escapes them, no whitespace. Members that are
// `undefined` are left out, as JSON leaves them out. Numbers are rounded to
// their field's precision before — what is written, not here.

/** The canonical JSON text of `value`. */
export function canonicalJson(value) {
  if (value === null) return 'null'
  switch (typeof value) {
    case 'boolean': return value ? 'true' : 'false'
    case 'number':
      if (!Number.isFinite(value)) throw new Error(`no JSON for ${value}`)
      return JSON.stringify(value)
    case 'string': return JSON.stringify(value)
    case 'object':
      if (Array.isArray(value)) return `[${value.map(v => (v === undefined ? 'null' : canonicalJson(v))).join(',')}]`
      return `{${Object.keys(value).filter(k => value[k] !== undefined).sort()
        .map(k => `${JSON.stringify(k)}:${canonicalJson(value[k])}`).join(',')}}`
    default: throw new Error(`no JSON for a ${typeof value}`)
  }
}

/** The content hash of `value`: "sha256:" and the SHA-256 of its canonical JSON, uncompressed. */
export const contentHash = (value) => `sha256:${sha256OfText(canonicalJson(value))}`

/** An id of UUID form drawn from `text` — the same text, the same id, on every device. */
export function derivedId(text) {
  const h = sha256OfText(text)
  // Version 8 (custom) and the RFC 4122 variant, so it reads as a UUID.
  const variant = ((parseInt(h[16], 16) & 0x3) | 0x8).toString(16)
  return `${h.slice(0, 8)}-${h.slice(8, 12)}-8${h.slice(13, 16)}-${variant}${h.slice(17, 20)}-${h.slice(20, 32)}`
}
