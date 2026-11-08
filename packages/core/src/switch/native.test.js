import { describe, expect, it } from 'vitest'
import { hydrateProjects } from '../project/persistenceUtils.js'
import { validateProject } from '../project/index.js'
import { rebuildSwitchSymbol, planSwitchDeletion, switchCouplings } from './index.js'
import { callNative } from '../native/index.js'
import { loadNativeFile } from '../native/node.js'
import { hasPek, loadPek, hasPekBestand, loadPekBestand } from '../../test/pekFixture.js'

// The switch module and the project's validation in Rust (decision 0020,
// step 4), held to the JavaScript on real planning data — read at run time,
// never recorded into the vectors (the data is not the repository's).

const ulp = (x) => (x === 0 ? Number.MIN_VALUE : 2 ** (Math.floor(Math.log2(Math.abs(x))) - 52))

/** Where two answers differ beyond 4·10⁻⁹ or four units in the last place: a path, or null. */
function differs(a, b, path = '') {
  if (typeof a === 'number' && typeof b === 'number') {
    if (a === b || (Number.isNaN(a) && Number.isNaN(b))) return null
    const d = Math.abs(a - b)
    return d <= 4e-9 || d <= 4 * ulp(Math.max(Math.abs(a), Math.abs(b))) ? null : `${path}: ${a} ≠ ${b}`
  }
  if (Array.isArray(a) && Array.isArray(b)) {
    if (a.length !== b.length) return `${path}: ${a.length} items ≠ ${b.length}`
    for (let i = 0; i < a.length; i++) { const d = differs(a[i], b[i], `${path}/${i}`); if (d) return d }
    return null
  }
  if (a && b && typeof a === 'object' && typeof b === 'object') {
    const keys = new Set([...Object.keys(a), ...Object.keys(b)].filter(k => a[k] !== undefined || b[k] !== undefined))
    for (const k of keys) { const d = differs(a[k], b[k], `${path}/${k}`); if (d) return d }
    return null
  }
  return a === b ? null : `${path}: ${JSON.stringify(a)} ≠ ${JSON.stringify(b)}`
}

const plain = (v) => JSON.parse(JSON.stringify(v, (_, x) => (x instanceof Map ? Object.fromEntries(x) : x)))
const records = () => [hasPek && loadPek(), hasPekBestand && loadPekBestand()].filter(Boolean)

describe.skipIf(!(hasPek || hasPekBestand) || !loadNativeFile())('the switch module in Rust, on the PEK data', { timeout: 60000 }, () => {
  it('validates each record alike', () => {
    for (const record of records()) {
      expect(differs(callNative('project.validateProject', [record]), plain(validateProject(record)))).toBeNull()
      const [hydrated] = hydrateProjects([record])
      expect(differs(callNative('project.validateProject', [hydrated]), plain(validateProject(hydrated)))).toBeNull()
    }
  })

  it('rebuilds every switch symbol alike', () => {
    for (const record of records()) {
      const [p] = hydrateProjects([record])
      const byId = Object.fromEntries(p.tracks.map(t => [t.id, t]))
      for (const sw of p.switches ?? []) {
        expect(differs(callNative('switch.rebuildSwitchSymbol', [sw, byId]), plain(rebuildSwitchSymbol(sw, byId))), sw.switchId).toBeNull()
      }
    }
  })

  it('plans every switch deletion alike', () => {
    for (const record of records()) {
      const [p] = hydrateProjects([record])
      for (const sw of p.switches ?? []) {
        expect(differs(callNative('switch.planSwitchDeletion', [sw, p.tracks]), plain(planSwitchDeletion(sw, p.tracks))), sw.switchId).toBeNull()
      }
    }
  })

  it('couples the turnouts alike', () => {
    for (const record of records()) {
      const [p] = hydrateProjects([record])
      const strip = (c) => ({
        we: c.we, lds: c.lds, ldsMain: c.ldsMain, ldsBranch: c.ldsBranch, offset: c.offset, cant: c.cant, reach: c.reach, end: c.end,
        sleepers: c.sleepers.map(s => ({ k: s.k, s: s.s, m: s.m, b: s.b, main: s.main, branch: s.branch, a: s.a, y: s.y })),
      })
      const rust = callNative('switch.switchCouplings', [p.tracks, p.switches ?? [], {}]).map(strip)
      const js = plain(switchCouplings(p.tracks, p.switches ?? [])).map(strip)
      expect(differs(rust, js)).toBeNull()
    }
  })
})
