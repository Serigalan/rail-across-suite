// project — the project record as a whole: the record the app works on (load,
// hydrate), the door to it from infra-data-1.0 documents (the one format,
// decision 0021), project settings, validation, what one step did. The
// door, keeping a record and the validation are the core's Rust module
// (decision 0020, step 10); loading one, the settings and the step labels —
// what the page asks for every track and history step it shows — stay here.
//
// This file is the module's public face: code outside packages/core/src/project/
// imports from here and nowhere else (tools/modules.mjs). What is not listed is
// the module's own business. See docs/modules/project.md.

import { viaRust } from '../native/index.js'

export {
  dehydrateProjects, FORMAT_NAME, heldKeptProject, heldProjectDocument, holdProject, hydrateProjects,
  layersWithoutDangling, parseProjectsPayload, parseTracksPayload, PayloadError, projectDocument, recordDomains, tracksDocument,
} from './persistenceUtils.js'
export {
  crsChoices, dominantSystem, moveTracksToPlane, newTrackEpsg, ownLineCategory, planeGroup,
  planeInSystem, previewTracksToPlane, projectSystem, projectSystemSetting, systemChoices,
  systemDeviates, withImportSystem, withLineCategory, zoneChoices,
} from './projectSettings.js'
export { describeStep, stepTrackIds } from './stepLabel.js'

/**
 * The findings of a record — what validateProject's rules 6 and 7 say of
 * it, { errors, warnings } — computed by the core's Rust module
 * (crates/project; decision 0020, step 10).
 */
export const validateProject = viaRust('project.validateProject')
/** The findings of `now` that `before` did not have (by their keys). */
export const newFindings = viaRust('project.newFindings')
