import { describe, it, expect, beforeAll } from 'vitest'
import { existsSync, readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { adoptCrossover, crossoverRequest, settleConnection } from '@rail-across/core/commands'
import { buildConnectionElements, SWITCH_TYPES } from '@rail-across/core/switch'
import { checkTrack } from '@rail-across/core/rules'
import { instantiateCrossover, runWith } from './crossoverRun'

// The search as the dialog runs it, through the module cargo built
// (crossover/target/…) — skipped where it has not been built.
const MODULE = fileURLToPath(new URL(
  '../../../../../../crossover/target/wasm32-unknown-unknown/release/rail_across_crossover_wasm.wasm', import.meta.url))
const built = existsSync(MODULE)

const EPSG = 25832
const P = (e, n) => ({ easting: e, northing: n, zone: EPSG })
const pick = (startUtm, bearing, length, along, radius = null) => ({
  zone: EPSG, startUtm, bearing, along, route: { length, r1: radius, r2: radius },
  cantStart: 0, cantEnd: 0, speed: 80, trackEnds: [false, false], name: 'x',
})
// Track 1 straight, track 2 an R 800 bending away from it: no one arc
// between the branches keeps the comparison radius at 80 km/h.
const picks = [pick(P(500000, 5600000), 0, 300, 150), pick(P(499995.5, 5599900), 1, 300, 140, -800)]
const SPEED = 80

/** Adopted, solved again and built: the elements the commit would write, and the rule check's errors of length and jump. */
function ruleErrorsOf(solution, form) {
  const adopted = adoptCrossover({ picks, speed: SPEED, solution, form })
  expect(adopted).not.toBeNull()
  const { result } = settleConnection({ picks, speed: SPEED, shift: adopted.shift, mids: adopted.mids, seed: adopted.seed, form })
  expect(result.valid).toBe(true)
  const { branch1, midEls, branch2 } = buildConnectionElements(result, SPEED)
  const out = checkTrack([...branch1, ...midEls, ...branch2])
  return [...out.elements, ...out.boundaries].flatMap(e => e.results ?? [])
    .filter(r => r.severity === 'error' && ['LP.EL.01', 'LP.KS.01'].includes(r.id))
}

describe.skipIf(!built)('the search of the extended switch connection, as WebAssembly', () => {
  let exports
  beforeAll(async () => { exports = await instantiateCrossover(readFileSync(MODULE)) })

  it('answers best first, and the best is built here without an error of length or jump', () => {
    const answer = runWith(exports, crossoverRequest({ picks, speed: SPEED }))
    expect(answer.solutions.length).toBeGreaterThan(1)
    expect(Math.abs(answer.solutions[0].s1)).toBeLessThan(1)
    expect(ruleErrorsOf(answer.solutions[0], null)).toEqual([])
  })

  it('keeps the elements given and builds each turnout of its own form', () => {
    const pair = ['760 – 1:14', '1200 – 1:18.5'].map(label => SWITCH_TYPES.find(f => f.label === label))
    const request = { ...crossoverRequest({ picks, speed: SPEED, form: pair }), elements: [{ type: 'transition' }, { type: 'arc' }] }
    const answer = runWith(exports, request)
    expect(answer.solutions).toHaveLength(1)
    const [sol] = answer.solutions
    expect(sol.template).toEqual(['transition', 'arc'])
    expect([sol.label, sol.label2]).toEqual(['760 – 1:14', '1200 – 1:18.5'])
    expect(ruleErrorsOf(sol, pair)).toEqual([])
  })

  it('says so where nothing closes', () => {
    const request = { ...crossoverRequest({ picks, speed: SPEED }), elements: [{ type: 'straight', length: 3 }, { type: 'arc', radius: 60 }] }
    expect(runWith(exports, request).error).toBe('no_solution')
  })
})
