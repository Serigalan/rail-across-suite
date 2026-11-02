import { storedBytes } from '../core/localStore'
import { WORKING, done as txDone, openDb, request as req } from '@rail-across/working-copy'

// The working copies in IndexedDB (phase 10): the suite's store, shared with
// the Signalling-Tool and the start page (@rail-across/working-copy), and what
// this tool alone does with the database — listing and emptying the stores of
// its version 1.
//
// DB `olt`, version 2, one object store in use:
//   workingCopies – keyPath `variantId`, one record per variant a user has
//                   opened: { variantId, projectId, base, basePayload,
//                   project, idLog, updatedAt } — the record being edited
//                   (dehydrated), the revision it rests on and that
//                   revision's record, for merging against.
//
// The stores of version 1 (`projects`, `images`) are left as they are and no
// longer read: there is no taking over of local projects (decision 95). The
// local storage dialog shows what they still hold and empties them on request.
//
// core/storage.js keeps the synchronous in-memory copy and writes it through
// workingCopies.js, which calls these; localStoreSections.js lists them in the
// local storage dialog. Nothing here is imported by UI code.

export { deleteWorkingCopy, getAllWorkingCopies, getWorkingCopy, openDb, putWorkingCopy } from '@rail-across/working-copy'

/** The stores this version no longer reads (see above), those that are there. */
const legacyStores = (db) => [...db.objectStoreNames].filter(name => name !== WORKING)

/**
 * What the stores of version 1 still hold: { records, bytes }.
 */
export async function legacyContents() {
  const db = await openDb()
  const names = legacyStores(db)
  if (!names.length) return { records: 0, bytes: 0 }
  const tx = db.transaction(names, 'readonly')
  const all = (await Promise.all(names.map(name => req(tx.objectStore(name).getAll())))).flat()
  await txDone(tx)
  return { records: all.length, bytes: all.reduce((sum, r) => sum + storedBytes(r), 0) }
}

export async function clearLegacyStores() {
  const db = await openDb()
  const names = legacyStores(db)
  if (!names.length) return
  const tx = db.transaction(names, 'readwrite')
  for (const name of names) tx.objectStore(name).clear()
  await txDone(tx)
}
