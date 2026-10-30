// signalling — the signalling domain's content: the catalogue of signal
// aspects, signalling and train protection systems (DB Ril 301 after PlanPro
// 1.10: Ks and shunting signals, PZB and ETCS balises), the check of what the
// schema cannot say — references, locations, what each signal may show,
// routes, overlaps, flank protection, balises, station boundaries — and the
// extent of the track sections over the net (decision 0011).
//
// This file is the module's public face: code outside packages/core/src/signalling/
// imports from here and nowhere else (tools/modules.mjs). See docs/signalling.md.

export {
  CATALOGUE_REF, SIGNAL_CATALOGUE, aspectFitsFrame, aspectOf, aspectSpeed, baliseKindOf, boardAspect, knownSystem, protectionSystemOf,
  screenOf, settingFits, settingsProblems, signalKindOf, symbolFits, titleIn,
} from './catalogue.js'
export { checkSignalling } from './checkSignalling.js'
export { netParts, trackSectionExtents } from './trackSections.js'
