// planpro — writing a project as a PlanPro 1.10 plan: its net, its switches and
// end marks, and its signalling (decision 0011). Computed by the core's Rust
// module (crates/planpro, write.rs). The suite has no PlanPro import: a plan
// holds too little of a project.
//
// This file is the module's public face: code outside packages/core/src/planpro/
// imports from here and nowhere else (tools/modules.mjs). See docs/planpro.md.

import { viaRust } from '../native/index.js'

/**
 * A project as a PlanPro plan: `writePlanPro(project, { signalling, now, version,
 * rulesDate, planning })` → `{ text, manifest, archive, report: { notes }, counts }`
 * — the plan (`content.xml`), its manifest, the `.planpro` file of both in
 * Base64, what was not written and why (`not_written`: collection, id, why) and
 * what PlanPro requires that the project does not say (`defaulted`: collection,
 * id, field, value), and how many objects of each PlanPro kind were written.
 * `project` is an infra-data-1.0 document or a record (made one first);
 * `signalling` the domain as the editor holds it, `now` the time stamp of the
 * file, `version` the suite's, `planning` { projectNumber, planner,
 * organisation, location, stage, index, number, informative, kind }.
 */
export const writePlanPro = viaRust('planpro.writePlanPro')
