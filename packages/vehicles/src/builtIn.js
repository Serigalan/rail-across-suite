import { fromOsrd } from './vehicles.js'

/**
 * OSRD's example vehicles (data/osrd/, LGPL-3.0 as the suite), taken into the
 * build: the suite's built-in vehicles, shown and copied, never changed. An
 * entry: { key: 'osrd:<file>', source: 'osrd', vehicle, notes, mayEdit: false }.
 */
const FILES = import.meta.glob('../data/osrd/*.json', { eager: true, import: 'default' })

/** Rolling stocks as built-in entries, by their names; one that is not a rolling stock is left out. */
export function builtInEntries(files = FILES) {
  return Object.entries(files).flatMap(([path, raw]) => {
    try {
      const { vehicle, notes } = fromOsrd(raw)
      const name = path.split('/').pop().replace(/\.json$/, '')
      return [{ key: `osrd:${name}`, source: 'osrd', vehicle, notes, mayEdit: false }]
    } catch {
      return []
    }
  })
}

export const BUILT_IN = builtInEntries()
