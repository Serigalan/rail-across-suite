// project — the project record as a whole: the oltjson-1.1 record the app
// works on (load, save, hydrate), the door to it from core 1.0 documents and
// oltjson-1.1 files, project settings, validation, what one step did. The
// door, keeping a record and the validation are the core's Rust module
// (decision 0020, step 10); loading one, the settings and the step labels —
// what the page asks for every track and history step it shows — stay here.
//
// This file is the module's public face: code outside packages/core/src/project/
// imports from here and nowhere else (tools/modules.mjs). What is not listed is
// the module's own business. See docs/modules/project.md.

import { viaRust } from '../native/index.js'

export {
  dehydrateProjects, FORMAT_NAME, hydrateProjects, parseProjectsPayload, parseTracksPayload,
  PayloadError, projectDocument, projectsPayload,
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
