import { catalogFiles, hashCatalogFiles } from '@rail-across/core/rules'

// The rule catalogues this bundle carries, as the bundler read them: the input
// of the hash the optimizer service is compared with (R0.1).
const RAW = import.meta.glob('../../../../packages/core/constraints/*.json', { query: '?raw', import: 'default', eager: true })

/** [{ name, text }] in name order. */
export const bundledCatalogFiles = () => catalogFiles(RAW)

let own = null
/** The hash of this bundle's catalogues (computed once). */
export function bundledCatalogHash() {
  if (!own) own = hashCatalogFiles(bundledCatalogFiles())
  return own
}
