import { describe, expect, it } from 'vitest'
import { readFileSync } from 'node:fs'
import * as Q from './queries.js'

// The small queries kept in JavaScript beside the crate are held to what the
// crate answers: its frozen vectors (crates/track/tests/vectors/track.json),
// recorded from this very JavaScript before the module took the rest over.

const cases = readFileSync(new URL('../../../../crates/track/tests/vectors/track.json', import.meta.url), 'utf8')
  .split('\n').map(l => l.trim().replace(/,$/, '')).filter(l => l.startsWith('{"op"')).map(l => JSON.parse(l))

// As the recorder called them where it called them otherwise than plainly.
const ADAPTED = {
  switchRoutes: (k) => Object.keys(Q.switchRoutePorts(k)),
  switchStatus: (x, ts) => Q.switchStatus(x, new Map(ts.map(t => [t.id, t]))),
}

/** Equal as JSON, numbers to their own last few units (the module computes them in Rust). */
function close(a, b) {
  if (typeof a === 'number' && typeof b === 'number') return Math.abs(a - b) <= 1e-9 * Math.max(1, Math.abs(a), Math.abs(b))
  if (Array.isArray(a)) return Array.isArray(b) && a.length === b.length && a.every((x, i) => close(x, b[i]))
  if (a && typeof a === 'object') {
    if (!b || typeof b !== 'object') return false
    const ka = Object.keys(a).filter(k => a[k] !== undefined), kb = Object.keys(b).filter(k => b[k] !== undefined)
    return ka.length === kb.length && ka.every(k => close(a[k], b[k]))
  }
  return a === b || (a == null && b == null)
}

describe('the track queries kept in JavaScript', () => {
  const ours = cases.filter(c => typeof Q[c.op.slice('track.'.length)] === 'function')
  it('are held to cases of the crate\'s vectors', () => {
    expect(new Set(ours.map(c => c.op)).size).toBeGreaterThan(20)
  })
  it('answer as the crate does, every case of theirs', () => {
    const wrong = []
    for (const c of ours) {
      const name = c.op.slice('track.'.length)
      const fn = ADAPTED[name] ?? Q[name]
      const args = Array.isArray(c.input) ? c.input : [c.input]
      const out = JSON.parse(JSON.stringify(fn(...args) ?? null))
      // A new switch's id is drawn afresh.
      const want = name === 'newSwitchFields' ? { ...c.output, switchId: out.switchId } : c.output
      if (!close(out, want ?? null)) wrong.push(`${c.op} ${JSON.stringify(c.input).slice(0, 80)}`)
    }
    expect(wrong).toEqual([])
  })
})
