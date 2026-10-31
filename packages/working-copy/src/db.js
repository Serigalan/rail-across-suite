import { FORMAT_NAME } from '@rail-across/core/project'

// The working copies in the browser's IndexedDB, a thin promise wrapper.
//
// DB `olt`, version 2 (the name and version the Layout-Tool gave it, so the
// copies kept before are still there), one object store in use:
//   workingCopies – keyPath `variantId`, one record per variant opened in a
//                   tool of the suite: { variantId, projectId, base,
//                   basePayload, project, idLog, updatedAt, format } — the
//                   record being edited (dehydrated), the revision it rests on
//                   and that revision's record, for merging against; the
//                   splits and joins since (idLog).
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

// A working copy is stored with the format its records are in (oltjson-1.1).
// One from before it is not read: it was not converted, and what the server
// keeps of that state is not opened either.
const current = (record) => record?.format === FORMAT_NAME

/** One working copy, or null. */
export async function getWorkingCopy(variantId) {
  const db = await openDb()
  const tx = db.transaction(WORKING, 'readonly')
  const record = await request(tx.objectStore(WORKING).get(variantId))
  await done(tx)
  return current(record) ? record : null
}

/** Every working copy. */
export async function getAllWorkingCopies() {
  const db = await openDb()
  const tx = db.transaction(WORKING, 'readonly')
  const records = await request(tx.objectStore(WORKING).getAll())
  await done(tx)
  return records.filter(current)
}

/** Keep a working copy, in the format its records are in. */
export async function putWorkingCopy(record) {
  const db = await openDb()
  const tx = db.transaction(WORKING, 'readwrite')
  tx.objectStore(WORKING).put({ ...record, format: FORMAT_NAME })
  await done(tx)
}

export async function deleteWorkingCopy(variantId) {
  const db = await openDb()
  const tx = db.transaction(WORKING, 'readwrite')
  tx.objectStore(WORKING).delete(variantId)
  await done(tx)
}
