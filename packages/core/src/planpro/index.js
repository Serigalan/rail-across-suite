// planpro — reading a PlanPro 1.10 plan: its net as the core's tracks,
// switches and end marks, its signalling as the domain (decision 0011), and
// placing that signalling on another net. Computed by the core's Rust module
// (decision 0020, step 10: crates/planpro), called with the arguments as the
// JavaScript took them.
//
// This file is the module's public face: code outside packages/core/src/planpro/
// imports from here and nowhere else (tools/modules.mjs). See docs/planpro.md.

import { viaRust } from '../native/index.js'

/** An XML text as its element tree: { name, attrs, children, text }; throws on what is no XML. */
export const parseXml = viaRust('planpro.parseXml')
/** A PlanPro interface file read: its net and signalling as the model the record is made of. */
export const readPlanPro = viaRust('planpro.readPlanPro')
/** A PlanPro file as a record, `{ id, title }` optional: { record, header, report }. */
export const planproRecord = viaRust('planpro.planproRecord')
/**
 * A plan's signalling laid on another net — every location carried from the
 * plan's tracks `from` onto `onto` ({ tracks, switches }), what lies on no
 * track of it left in the plane: { signalling, carried }.
 */
export const placeSignalling = viaRust('planpro.placeSignalling')
