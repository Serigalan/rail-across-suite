import { strFromU8, strToU8, unzipSync, zipSync } from 'fflate'
import { PayloadError } from './payloadError.js'

// The container `.railacross` (decision 0002): a file rail-across-1.0 zipped,
// as OpenDocument and EPUB are — its first entry `mimetype`, stored
// uncompressed and without an extra field, so the media type stands at byte
// 38 for a tool that tells files by their first bytes; then the file itself
// as `project.json`, compressed. Unzipped, the JSON is the file byte for byte.

/** The container's extension, media type, and the entry that holds the file. */
export const CONTAINER_EXTENSION = '.railacross'
export const CONTAINER_TYPE = 'application/vnd.rail-across+zip'
export const CONTAINER_ENTRY = 'project.json'

// The local header of a zip entry is 30 bytes, its name follows.
const NAME_AT = 30
const TYPE_AT = NAME_AT + 'mimetype'.length

/** Whether the bytes are a container: a zip whose first entry names the container's media type. */
export function isContainer(bytes) {
  const b = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes)
  return b.length > TYPE_AT + CONTAINER_TYPE.length
    && b[0] === 0x50 && b[1] === 0x4b && b[2] === 0x03 && b[3] === 0x04
    && strFromU8(b.subarray(NAME_AT, TYPE_AT)) === 'mimetype'
    && strFromU8(b.subarray(TYPE_AT, TYPE_AT + CONTAINER_TYPE.length)) === CONTAINER_TYPE
}

/** A file rail-across-1.0 (projectFile) as a container's bytes. */
export function toContainer(file) {
  return zipSync({
    mimetype: [strToU8(CONTAINER_TYPE), { level: 0 }],
    [CONTAINER_ENTRY]: [strToU8(JSON.stringify(file)), { level: 9 }],
  })
}

/**
 * The JSON text of a chosen file's bytes: the file a container holds, else
 * the bytes themselves as UTF-8 — a `.railacross.json` and every other JSON
 * file read as before. Throws a PayloadError `invalid_payload` for a
 * container without its file.
 */
export function fileText(bytes) {
  const b = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes)
  if (!isContainer(b)) return strFromU8(b)
  let entry
  try {
    entry = unzipSync(b, { filter: (f) => f.name === CONTAINER_ENTRY })[CONTAINER_ENTRY]
  } catch (err) {
    throw new PayloadError('invalid_payload', [`container: ${err.message}`])
  }
  if (!entry) throw new PayloadError('invalid_payload', [`container: no ${CONTAINER_ENTRY}`])
  return strFromU8(entry)
}
