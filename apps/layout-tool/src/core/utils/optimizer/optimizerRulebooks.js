import { CATALOGUE } from '@rail-across/core/rules'
import { runOptimizer } from './optimizerJob'

/**
 * The rule catalogue the optimizer module applies, as the regelwerk popup
 * asks for it: { list() → { rulebooks, drift }, get(id) → regelwerk | null }
 * — the one this bundle carries (the module is handed it, so there is no
 * drift), with the limits a run is held to at Regelwert and Ermessensgrenze
 * as the module reads them out of it.
 */
export const optimizerRulebooks = {
  list: async () => ({
    rulebooks: [{ id: CATALOGUE.catalog.id, name: CATALOGUE.catalog.title, version: CATALOGUE.catalog.katalog_version }],
    drift: false,
  }),
  get: async (id) => {
    if (id !== CATALOGUE.catalog.id) return null
    try {
      return await runOptimizer('limits', {})
    } catch {
      return null
    }
  },
}
