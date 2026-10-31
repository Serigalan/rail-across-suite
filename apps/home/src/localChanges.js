import { useEffect, useState } from 'react'
import { getAllWorkingCopies, localChangeCount } from '@rail-across/working-copy'

/**
 * The variants whose working copy in this browser holds changes not checked
 * in, from any tool of the suite: { variantId: count }. Read when the page
 * opens and whenever its tab comes back; nothing where the browser keeps no
 * working copies.
 */
export function useLocalChanges() {
  const [counts, setCounts] = useState({})
  useEffect(() => {
    let alive = true
    const read = () => getAllWorkingCopies()
      .then(all => {
        if (!alive) return
        setCounts(Object.fromEntries(all.map(wc => [wc.variantId, localChangeCount(wc)]).filter(([, n]) => n > 0)))
      })
      .catch(() => { /* no IndexedDB here: nothing to say */ })
    read()
    const onShow = () => { if (document.visibilityState === 'visible') read() }
    document.addEventListener('visibilitychange', onShow)
    return () => { alive = false; document.removeEventListener('visibilitychange', onShow) }
  }, [])
  return counts
}
