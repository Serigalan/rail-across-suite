import { createContext, useContext } from 'react'

/**
 * What is changed and not committed yet: { open, what, commit, cancel, t } —
 * `what` the change in words. Every panel of a section that changes the
 * domain has it, read-only none.
 */
export const DraftContext = createContext(null)

/** What is changed and not committed yet, where the panel has it; else null. */
export const useDraft = () => useContext(DraftContext)
