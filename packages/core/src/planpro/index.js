// planpro — reading a PlanPro 1.10 plan: its net as the core's tracks,
// switches and end marks, its signalling as the domain (decision 0011), and
// placing that signalling on another net.
//
// This file is the module's public face: code outside packages/core/src/planpro/
// imports from here and nowhere else (tools/modules.mjs). See docs/planpro.md.

export { placeSignalling, planproRecord, readPlanPro } from './planpro.js'
export { parseXml } from './xml.js'
