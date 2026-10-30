// signalling — the signalling domain's content: the catalogue of signal
// aspects (DB Ril 301, Ks with PZB) and the check of what the schema cannot
// say — references, locations, what each signal may show, routes, overlaps,
// flank protection, PZB, station boundaries (decision 0011).
//
// This file is the module's public face: code outside packages/core/src/signalling/
// imports from here and nowhere else (tools/modules.mjs). See docs/signalling.md.

export { CATALOGUE_REF, SIGNAL_CATALOGUE, aspectOf, signalKindOf } from './catalogue.js'
export { checkSignalling } from './checkSignalling.js'
