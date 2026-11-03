import { KATALOG } from '@rail-across/core/rules'
import { runOptimizer } from './optimizerJob'

/**
 * The rule catalogue the optimizer module applies, as the regelwerk popup
 * asks for it: { list() → { regelwerke, drift }, get(id) → regelwerk | null }
 * — the one this bundle carries (the module is handed it, so there is no
 * drift), with the limits a run is held to at Regelwert and Ermessensgrenze
 * as the module reads them out of it.
 */
export const optimizerRegelwerke = {
  list: async () => ({
    regelwerke: [{ id: KATALOG.catalog.id, name: KATALOG.catalog.title, version: KATALOG.catalog.katalog_version }],
    drift: false,
  }),
  get: async (id) => {
    if (id !== KATALOG.catalog.id) return null
    try {
      return await runOptimizer('limits', {})
    } catch {
      return null
    }
  },
}
