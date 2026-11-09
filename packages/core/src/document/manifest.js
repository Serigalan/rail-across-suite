import { callNative } from '../native/index.js'

/**
 * The manifest of a document: per domain its version, its schema and the hash
 * of its fields and of each collection — the hash tree of the data model
 * (section 5.2). The unit of storage is the collection (S-R1): a revision that
 * leaves a collection as it was names the same hash for it. Computed by the
 * core's Rust module (crates/doc).
 */
export const manifestOf = (doc) => callNative('document.manifestOf', doc)

/** The hash of a document as a whole: its manifest's. */
export const documentHash = (doc) => callNative('document.documentHash', doc)
