// signalling — the signalling domain's content: the catalogue of signal
// aspects, signalling and train protection systems (DB Ril 301 after PlanPro
// 1.10: Ks and shunting signals, PZB and ETCS balises), the check of what the
// schema cannot say — references, locations, what each signal may show,
// routes, overlaps, flank protection, balises, station boundaries — and the
// extent of the track sections over the net (decision 0011).
//
// The check and the track sections are computed by the core's Rust module
// (decision 0020, step 10: crates/signalling). The catalogue's lookups, the
// overlap lengths and the speeds stay JavaScript: the page asks them for every
// signal and route it draws, and each call would carry the whole domain across
// (12 µs a call at the least) — over the catalogue that both read
// (constraints/db-ril-301.json).
//
// This file is the module's public face: code outside packages/core/src/signalling/
// imports from here and nowhere else (tools/modules.mjs). See docs/signalling.md.

import { asMap, viaRust } from '../native/index.js'

export {
  CATALOGUE_REF, SIGNAL_CATALOGUE, aspectFitsFrame, aspectOf, aspectSpeed, baliseKindOf, boardAspect, knownSystem, protectionSystemOf,
  screenOf, settingFits, settingsProblems, signalKindOf, symbolFits, titleIn,
} from './catalogue.js'
export { boundaryStartSpeed, gradientAlong, overlapLength, routeSpeedOf, signallingSpeedOf } from './overlaps.js'
export { branchSpeedOnRun } from './switchSpeeds.js'

/** The findings of the domain on the core it stands on, `{ core, signalling }`: { errors, warnings }. */
export const checkSignalling = viaRust('signalling.checkSignalling')

/** The parts of the net between detection points, `only` the points to cut at where given. */
export const netParts = viaRust('signalling.netParts')

/** Each track section's extent over the net: Map(section id → { parts, matches, missing }). */
export const trackSectionExtents = viaRust('signalling.trackSectionExtents', asMap)
