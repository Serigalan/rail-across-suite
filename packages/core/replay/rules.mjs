// The rules crate's vectors (crates/rules): the limits read out of the bundled
// catalogue, the cant rules, element lengths, and the catalogue applied to
// element chains — checkTrack, elementVerdict, the highest speeds — recorded
// from the JavaScript on chains laid by the factory from a fixed seed.
import * as r from '../src/rules/index.js'
import { elementVerdict } from '../src/rules/alignmentCheck.js'
import { ruleMaxSpeed } from '../src/rules/speed.js'
import { catalogSpeedRange } from '../src/rules/ruleCatalogue.js'
import { maxSpeedFor } from '../src/rules/cant.js'
import { arcFrom, straightFrom, transitionElement } from '../src/geometry/index.js'

const clone = (v) => JSON.parse(JSON.stringify(v))
function rng(seed) {
  return () => {
    seed |= 0; seed = (seed + 0x6D2B79F5) | 0
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed)
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}
const call = (op, input, fn) => ({ op, input: clone(input), output: clone(fn()) })
// What the rules read of an element — the rest is display geometry.
const READ = ['elementType', 'transitionType', 'radius', 'r1', 'r2', 'length', 'speed', 'cant', 'cantStart', 'cantEnd', 'switchId', 'switchBranch', 'cantException']
const bare = (el) => Object.fromEntries(READ.filter(k => el[k] !== undefined).map(k => [k, el[k]]))

export default function record() {
  const rand = rng(8000110)
  const limits = []
  limits.push(call('rules.constants', null, () => ({
    MAX_CANT: r.MAX_CANT, CANT_STEP: r.CANT_STEP, MAX_SWITCH_CANT: r.MAX_SWITCH_CANT, MAX_SWITCH_CANT_DEF: r.MAX_SWITCH_CANT_DEF,
    MAX_SWITCH_CANT_EXCEPTION: r.MAX_SWITCH_CANT_EXCEPTION, AUTO_CANT_MODEL: r.AUTO_CANT_MODEL, catalogSpeedRange,
  })))
  for (const v of [0, 20, 40, 60, 80, 100, 120, 140, 150, 155, 160, 200, 230, 250, 300]) {
    limits.push(call('rules.minElementLength', { v }, () => r.minElementLength(v)))
    limits.push(call('rules.comparisonRadiusLimits', { v }, () => r.comparisonRadiusLimits(v)))
  }
  for (let i = 0; i < 60; i++) {
    const speed = 20 + Math.round(rand() * 46) * 5
    const radius = (rand() < 0.5 ? -1 : 1) * (150 + rand() * 3000)
    const cant = Math.round((rand() - 0.2) * 32) * 5 * Math.sign(radius)
    const el = { speed, radius, cant, ...(i % 4 === 0 ? { switchBranch: 'branch', switchId: 's' } : {}), ...(i % 8 === 0 ? { cantException: ' Begründung ' } : {}) }
    limits.push(call('rules.computeAutoC', { speed, radius }, () => r.computeAutoC(speed, radius)))
    limits.push(call('rules.standardCant', { speed, radius, inSwitch: i % 3 === 0 }, () => r.standardCant(speed, radius, { inSwitch: i % 3 === 0 })))
    limits.push(call('rules.computeSwitchCant', { speed, radius }, () => r.computeSwitchCant(speed, radius)))
    limits.push(call('rules.computeCantDef', { speed, radius, cant }, () => r.computeCantDef(speed, radius, cant)))
    limits.push(call('rules.computeCantDefSigned', { speed, radius, cant }, () => r.computeCantDefSigned(speed, radius, cant)))
    limits.push(call('rules.limitCantDef', { el }, () => r.limitCantDef(el)))
    limits.push(call('rules.cantDefLevel', { el, cantDef: 100 + i }, () => r.cantDefLevel(el, 100 + i)))
    limits.push(call('rules.cantLimit', el, () => r.cantLimit(el)))
    limits.push(call('rules.clampCant', { value: cant * 1.37, el }, () => r.clampCant(cant * 1.37, el)))
    limits.push(call('rules.worstCantOf', { ...el, cantStart: cant / 2, cantEnd: cant }, () => r.worstCantOf({ ...el, cantStart: cant / 2, cantEnd: cant })))
    limits.push(call('rules.cantExceedsLimit', el, () => r.cantExceedsLimit(el)))
    limits.push(call('rules.cantExceptionFields', { cant, reason: i % 2 ? ' weil ' : '' }, () => r.cantExceptionFields(cant, i % 2 ? ' weil ' : '')))
    limits.push(call('rules.switchCantError', { cant, reason: i % 2 ? 'weil' : null }, () => r.switchCantError(cant, i % 2 ? 'weil' : null)))
    limits.push(call('rules.maxSpeedFor', { el, radius, cant }, () => maxSpeedFor(el, radius, cant)))
  }
  for (const draft of ['', '-', '63', '  127 ', 'x', '1e2', 42, -200]) {
    limits.push(call('rules.cantFromInput', { draft, min: -160, max: 160 }, () => r.cantFromInput(draft, -160, 160)))
  }

  const chains = []
  for (let i = 0; i < 10; i++) {
    let at = { easting: 600000 + rand() * 2000, northing: 5600000 + rand() * 2000, zone: 25832 }
    let bearing = rand() * 360
    const v = [0, 60, 80, 100, 120, 160, 200][i % 7]
    const R = (rand() < 0.5 ? -1 : 1) * (250 + rand() * 1500)
    const u = Math.round(r.computeAutoC(v || 80, R) / 5) * 5
    const els = []
    const push = (el) => { els.push(el); at = { easting: el.endNode[0], northing: el.endNode[1], zone: 25832 }; bearing = el.endBearing ?? el.bearing }
    push(straightFrom(at, bearing, 20 + rand() * 200, { speed: v }))
    push(transitionElement(at, bearing, 20 + rand() * 120, null, R, { speed: v, transitionType: i % 3 ? 'clothoid' : 'bloss' }).element)
    push(arcFrom(at, bearing, 10 + rand() * 150, R, { speed: v, cant: u, ...(i % 4 === 1 ? { switchId: 'w1' } : {}) }))
    push(transitionElement(at, bearing, 20 + rand() * 120, R, -R * 1.3, { speed: i % 5 ? v : v + 40 }).element)
    push(arcFrom(at, bearing, 10 + rand() * 150, -R * 1.3, { speed: v, cant: -u, cantStart: -u / 2, cantEnd: -u }))
    push(straightFrom(at, bearing, 5 + rand() * 60, { speed: v, ...(i % 4 === 1 ? { switchId: 'w1' } : {}) }))
    chains.push(els.map(bare))
  }
  const checks = []
  for (const elements of chains) {
    checks.push(call('rules.checkTrack', { elements }, () => r.checkTrack(elements)))
    const context = [{ elementType: 'straight', radius: null, length: 40 }, { elementType: 'arc', radius: 900, length: 30 }]
    checks.push(call('rules.checkTrack', { elements, context }, () => r.checkTrack(elements, { context })))
    checks.push(call('rules.hasRuleError', { elements }, () => r.hasRuleError(elements)))
    for (let i = 0; i < elements.length; i++) {
      checks.push(call('rules.elementVerdict', { elements, i }, () => elementVerdict(elements, i)))
      checks.push(call('rules.governing', { elements, i }, () => r.governing(elements, i)))
    }
    for (const i of [1, 2, 4]) {
      checks.push(call('rules.ruleMaxSpeed', { elements, i }, () => ruleMaxSpeed(elements, i)))
      checks.push(call('rules.ruleMaxSpeed', { elements, i, cap: 100 }, () => ruleMaxSpeed(elements, i, { cap: 100 })))
    }
    checks.push(call('rules.maxSpeeds', { elements, cap: 120 }, () => r.maxSpeeds(elements, 120)))
    const results = r.checkTrack(elements).perElement.flatMap(e => e.results)
    checks.push(call('rules.fieldFindings', results, () => r.fieldFindings(results)))
  }
  return { limits, checks }
}
