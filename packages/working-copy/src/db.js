import { FORMAT_NAME, parseProjectsPayload, projectDocument } from '@rail-across/core/project'

// The working copies in the browser's IndexedDB, a thin promise wrapper.
//
// DB `olt`, version 2 (the name and version the Layout-Tool gave it, so the
// copies kept before are still there), one object store in use:
//   workingCopies – keyPath `variantId`, one entry per variant opened in a
//                   tool of the suite: { variantId, projectId, base,
//                   baseDocument, document, idLog, updatedAt, format } — the
//                   state being edited and that of the revision it rests on,
//                   for merging against, each as its infra-data-1.0 document
//                   (decision 0021: the one format); the splits and joins
//                   since (idLog).
//
// The suite's tools read and write a copy as { …, basePayload, project } with
// the records the app works on; the documents are this file's business.
//
// The stores of version 1 (`projects`, `images`) are the Layout-Tool's to
// list and empty; nothing here reads them.

const DB_NAME = 'olt'
const DB_VERSION = 2
export const WORKING = 'workingCopies'

let dbPromise = null

/** A request as a promise. */
export function request(r) {
  return new Promise((resolve, reject) => {
    r.onsuccess = () => resolve(r.result)
    r.onerror = () => reject(r.error)
  })
}

/** A transaction's end as a promise. */
export function done(tx) {
  return new Promise((resolve, reject) => {
    tx.oncomplete = () => resolve()
    tx.onabort = tx.onerror = () => reject(tx.error)
  })
}

/** The database, opened once. */
export function openDb() {
  if (dbPromise) return dbPromise
  dbPromise = new Promise((resolve, reject) => {
    const r = globalThis.indexedDB.open(DB_NAME, DB_VERSION)
    r.onupgradeneeded = () => {
      const db = r.result
      if (!db.objectStoreNames.contains(WORKING)) db.createObjectStore(WORKING, { keyPath: 'variantId' })
    }
    r.onsuccess = () => resolve(r.result)
    r.onerror = () => reject(r.error)
  })
  return dbPromise
}

// A copy kept before decision 0021 holds the records themselves, marked
// oltjson-1.1; it is read once as it is and kept again as documents. One of
// any other format is not read: what the server keeps of that state is not
// opened either.
const OLD_FORMAT = 'oltjson-1.1'

// A record's document, made once: the records do not change (the base of a
// copy is written again with every step of the copy).
const documents = new WeakMap()
function documentOf(record) {
  if (!record) return undefined
  let doc = documents.get(record)
  if (!doc) {
    doc = projectDocument(record)
    documents.set(record, doc)
  }
  return doc
}
const recordOf = (doc) => (doc ? parseProjectsPayload(doc).projects[0] : undefined)

/**
 * A copy as it is kept: its records as documents. A tool that has the
 * project's document already (the Layout-Tool's store, from the core's module)
 * hands it as `document` instead of the record.
 */
function stored({ project, basePayload, document, ...rest }) {
  return { ...rest, document: document ?? documentOf(project), baseDocument: documentOf(basePayload), format: FORMAT_NAME }
}

/** A kept copy as the tools read it — its documents through the door — or null where it cannot be read. */
function copyOf(entry) {
  // Read through the door as well, so that it holds the record's words of now.
  if (entry?.format === OLD_FORMAT) return copyOf(stored((({ format: _format, ...copy }) => copy)(entry)))
  if (entry?.format !== FORMAT_NAME) return null
  const { document, baseDocument, format: _format, ...rest } = entry
  try {
    return { ...rest, project: recordOf(document), basePayload: recordOf(baseDocument) }
  } catch (err) {
    console.warn(`working copy ${entry.variantId} not read: ${err.code ?? err.message}`)
    return null
  }
}

/** The copies of `entries` kept before decision 0021, kept again as documents. */
async function migrate(entries) {
  const old = entries.filter(e => e?.format === OLD_FORMAT)
  if (!old.length) return
  const db = await openDb()
  const tx = db.transaction(WORKING, 'readwrite')
  for (const { format: _format, ...copy } of old) tx.objectStore(WORKING).put(stored(copy))
  await done(tx)
}

/** One working copy, or null. */
export async function getWorkingCopy(variantId) {
  const db = await openDb()
  const tx = db.transaction(WORKING, 'readonly')
  const entry = await request(tx.objectStore(WORKING).get(variantId))
  await done(tx)
  await migrate([entry])
  return copyOf(entry)
}

/** Every working copy. */
export async function getAllWorkingCopies() {
  const db = await openDb()
  const tx = db.transaction(WORKING, 'readonly')
  const entries = await request(tx.objectStore(WORKING).getAll())
  await done(tx)
  await migrate(entries)
  return entries.map(copyOf).filter(Boolean)
}

/** Keep a working copy: { variantId, projectId, base, basePayload, project | document, idLog, … }. */
export async function putWorkingCopy(copy) {
  const entry = stored(copy)
  const db = await openDb()
  const tx = db.transaction(WORKING, 'readwrite')
  tx.objectStore(WORKING).put(entry)
  await done(tx)
}

export async function deleteWorkingCopy(variantId) {
  const db = await openDb()
  const tx = db.transaction(WORKING, 'readwrite')
  tx.objectStore(WORKING).delete(variantId)
  await done(tx)
}
