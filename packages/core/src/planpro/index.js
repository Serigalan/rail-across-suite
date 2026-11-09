// planpro — writing a project as a PlanPro 1.10 plan: its net, its switches and
// end marks, and its signalling (decision 0011). Computed by the core's Rust
// module (crates/planpro, write.rs). The suite has no PlanPro import: a plan
// holds too little of a project.
//
// This file is the module's public face: code outside packages/core/src/planpro/
// imports from here and nowhere else (tools/modules.mjs). See docs/planpro.md.

import { viaRust } from '../native/index.js'

/**
 * A project as a PlanPro plan: `writePlanPro(record, { signalling, now, version })`
 * → `{ text, report: { notes }, counts }` — the XML (a `.ppxml`), what could not
 * be written and why (`not_written`: collection, id, why), and how many objects
 * of each PlanPro kind were. `record` holds `id`, `title`, `tracks`, `switches`
 * and `endMarks`; `signalling` is the domain (else the record's), `now` the time
 * stamp of the file, `version` the suite's.
 */
export const writePlanPro = viaRust('planpro.writePlanPro')
