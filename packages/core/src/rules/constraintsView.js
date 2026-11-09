// The limits the optimizer reports it runs at set beside the app's own — the
// reading half of RulebookOverlay.jsx —, and the physics file turned into
// rows, which its tests hold to the arithmetic. Plain functions, not JSX, so
// they run in the Node test suite without a DOM.
//
// Physics has no route on the service and needs none: since the constraints
// were gathered in packages/core/constraints/ it is one of the repo's own rulebooks,
// read by the app at build time and by the optimizer's verification straight
// off disk. Not a copy — the import below is that file, which is why there is
// nothing here that could drift from it.

import physics from '../../constraints/physics.json' with { type: 'json' }
import {
  catalogLimit, catalogSpeedRange, evaluateRule, IN_SWITCH_AREA, CATALOGUE, ruleById, severityRank,
} from './ruleCatalogue.js'

export const PHYSICS = physics

// Where each physics constant is read, the same app knowledge regelwerkView's
// WHERE_USED carries — and with the same caveat the file states about itself:
// it is the readable derivation the kernel's literals are checked against
// (verify.py), not something the kernel loads. Two of the three constants do
// not even reach the kernel as themselves; they are the derivation of the
// third.
const PHYSICS_WHERE_USED = {
  ueberhoehungsfehlbetrag_koeffizient:
    'Optimierer-Kernel (geometry.py) und App — Fehlbetrag und V_max im Trackeditor',
  wirksame_spurweite: 'nur Herleitung — steckt im Koeffizienten 11,8',
  erdbeschleunigung: 'nur Herleitung — steckt im Koeffizienten 11,8',
}

// The two levels a run can be held to, and the worst severity each admits —
// the same pairing as the optimizer's grenzen.py (STUFEN).
export const LIMIT_SEVERITIES = { reg: 'hint', discretion: 'warning' }

// The rule scopes a level lets a run past — grenzen.py's TOLERIERT: at the
// Ermessensgrenze a run proposes Bloss transitions although LP.UB.02 calls one
// on this app's straight ramp a Sonderfall (ROADMAP, decision 57).
const TOLERATED = { reg: [], discretion: ['cant_ramp'] }

const DESIGN_SPEEDS = (() => {
  const out = []
  for (let v = catalogSpeedRange.min; v <= catalogSpeedRange.max; v++) {
    if (evaluateRule(ruleById('LP.ALL.02'), { 'element.design_speed': v }).severity === 'ok') out.push(v)
  }
  return out
})()

const distinct = (values) => [...new Set(values)].sort((a, b) => a - b)

/**
 * The same limits as the app reads them out of its own bundled catalogue, per
 * level — what the service's numbers are held against. Not a second
 * derivation of the optimizer's: the threshold a level stands for is named
 * outright (`reg`, `discretion`), which is all a display needs; the service
 * finds it from the severities, and a disagreement between the two is exactly
 * what this column exists to show.
 */
function appLimits(level) {
  const kb05 = catalogLimit('LP.KB.05', level, { 'element.cant': 0 }, IN_SWITCH_AREA)
  const kb06 = catalogLimit('LP.KB.06', 'max', { 'physics.u_f': 0 }, IN_SWITCH_AREA)
  const uf = (v) => catalogLimit('LP.KB.02', level, { 'element.design_speed': v, 'physics.u_f': 0 })
  const allowed = severityRank(LIMIT_SEVERITIES[level])
  const formOk = (form) => TOLERATED[level].includes(ruleById('LP.UB.02').applies_to.scope)
    || severityRank(evaluateRule(ruleById('LP.UB.02'), {
      'model.ramp_on_transition': true, 'model.ramp_form_matches': form === 'clothoid',
    }).severity) <= allowed
  const uMax = catalogLimit('LP.KB.01', 'max', { 'element.cant': 0 })
  return {
    geschwindigkeiten: [catalogSpeedRange.min, catalogSpeedRange.max],
    uMax,
    uMaxWeiche: Math.min(uMax, kb05),
    uStep: catalogLimit('LP.KB.03', 'step', { 'element.cant': 0 }),
    ufMax: distinct(DESIGN_SPEEDS.map(uf)),
    ufMaxWeiche: distinct(DESIGN_SPEEDS.map(v => Math.min(uf(v), kb06))),
    uebergangsbogen: ['clothoid', 'bloss'].filter(formOk),
    schlechtesteStufe: LIMIT_SEVERITIES[level],
  }
}

// One row per limit the service reports, in the order a designer reads them.
const LIMIT_ROWS = [
  { key: 'geschwindigkeiten', label: 'constraints_limit_speeds', unit: 'km/h', join: '–' },
  { key: 'uMax', label: 'constraints_limit_u', unit: 'mm' },
  { key: 'uMaxWeiche', label: 'constraints_limit_u_switch', unit: 'mm' },
  { key: 'uStep', label: 'constraints_limit_u_step', unit: 'mm' },
  { key: 'ufMax', label: 'constraints_limit_uf', unit: 'mm', join: ' / ' },
  { key: 'ufMaxWeiche', label: 'constraints_limit_uf_switch', unit: 'mm', join: ' / ' },
  { key: 'uebergangsbogen', label: 'constraints_limit_forms', kind: 'forms' },
  { key: 'schlechtesteStufe', label: 'constraints_limit_worst', kind: 'severity' },
]

/**
 * What GET /rulebooks/<id> says a run is held to, as table rows beside the
 * app's own reading of the bundled catalogue: [{ label, unit, kind, join,
 * reg: { served, app, mismatch }, discretion: { … } }]. A service deployed
 * from an older commit than this bundle is the disagreement the drift tests on
 * disk cannot see — this is where it shows.
 */
export function optimizerLimitRows(served) {
  const app = { reg: appLimits('reg'), discretion: appLimits('discretion') }
  return LIMIT_ROWS.map(row => {
    const out = { label: row.label, unit: row.unit ?? '', kind: row.kind ?? 'number', join: row.join ?? ', ' }
    for (const level of Object.keys(LIMIT_SEVERITIES)) {
      const s = served?.grenzwerte?.[level]?.[row.key]
      const a = app[level][row.key]
      out[level] = {
        served: s ?? null,
        app: a,
        mismatch: s !== undefined && s !== null && JSON.stringify(s) !== JSON.stringify(a),
      }
    }
    return out
  })
}

/** The catalogue version the app bundles, to hold the service's against. */
export const BUNDLED_CATALOGUE_VERSION = CATALOGUE.catalog.katalog_version

/**
 * Evaluates a `*_calc` expression from physics.json — never anything a user
 * typed or a service answered, always this repo's own file, which is what
 * makes `Function` safe to reach for here. `variables` names the values the
 * expression's identifiers resolve to; `sqrt` is the one function name a
 * formula may call.
 */
export function evalFormula(expr, variables) {
  const names = Object.keys(variables)
  const fn = new Function('sqrt', ...names, `"use strict"; return (${expr});`)
  return fn(Math.sqrt, ...names.map(n => variables[n]))
}

/**
 * physics.json as rows: `constants` (one per { wert, ... } entry, in file
 * order) and `profile` (the transition curve profiles, which state a curvature
 * function instead of a value). Values keep their JSON type; formatting is the
 * caller's job.
 *
 * A formula travels twice, and neither is a copy of the other: `*_calc` is a
 * bare, evaluable expression (no units, no prose — `evalFormula` below runs
 * it), `*_mathml` the set formula for a reader of the file. No page shows
 * them; the rows exist so a test can hold the file to the kernel's
 * own arithmetic (see constraintsView.test.js and tests/verify.py's mirror of
 * it) instead of only the constant the formula produces.
 */
export function flattenPhysics(doc) {
  const constants = []
  for (const [key, node] of Object.entries(doc ?? {})) {
    if (!node || typeof node !== 'object' || Array.isArray(node)) continue
    if (!('wert' in node)) continue
    constants.push({
      path: key,
      wert: node.wert,
      einheit: node.einheit ?? '',
      formulaCalc: node.formel_calc ?? '',
      formulaMathml: node.formel_mathml ?? '',
      herleitung: node.herleitung ?? '',
      warum: node.warum ?? '',
      whereUsed: PHYSICS_WHERE_USED[key] ?? 'Optimierer',
    })
  }
  const profile = Object.entries(doc?.uebergangsbogenprofile ?? {}).map(([key, node]) => ({
    key,
    name: node.name ?? key,
    kruemmung: node.kruemmung ?? '',
    curvatureCalc: node.kruemmung_calc ?? '',
    curvatureMathml: node.kruemmung_mathml ?? '',
    warum: node.warum ?? '',
  }))
  return { constants, profile }
}
