import { getWorkingCopy, putWorkingCopy } from '@rail-across/working-copy'

// The variant's working copy in this browser — the same one the Layout-Tool
// works on (@rail-across/working-copy): what this tool changes is kept there
// first, never only in the page, and goes to the server when it is checked in.
// A copy is { base, basePayload, project, idLog }: the revision it rests on and
// that revision's record, the record being edited, the splits and joins since.

/** The variant's working copy where this browser keeps one, else its head as a fresh one. */
export async function openCopy(variantId, head) {
  const stored = await getWorkingCopy(variantId).catch(() => null)
  if (stored?.project && stored.base && stored.basePayload) {
    return { base: stored.base, basePayload: stored.basePayload, project: stored.project, idLog: stored.idLog ?? [] }
  }
  return freshCopy(head)
}

/** A copy that is the head itself: nothing changed against it. */
export const freshCopy = (head) => ({ base: head.revision, basePayload: head.record, project: head.record, idLog: [] })

/** Keep the copy; where the browser keeps nothing it lives on in the page only. */
export const keepCopy = (variantId, copy) => putWorkingCopy({
  variantId, projectId: copy.project.id, base: copy.base, basePayload: copy.basePayload, project: copy.project,
  idLog: copy.idLog ?? [], updatedAt: new Date().toISOString(),
}).catch(() => { /* no IndexedDB here */ })
