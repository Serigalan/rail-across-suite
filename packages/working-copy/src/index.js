// @rail-across/working-copy — the local working copy of a variant (README.md):
// kept in the browser's IndexedDB, one per variant, the same for every tool of
// the suite, so what one tool changed the next one opens with, and the start
// page can say which variants hold changes not checked in.

export { deleteWorkingCopy, done, getAllWorkingCopies, getWorkingCopy, openDb, putWorkingCopy, request, WORKING } from './db.js'
export { domainChanges, localChangeCount, localChanges } from './changes.js'
