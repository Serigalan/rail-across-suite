// project — the project record as a whole: the oltjson-1.1 format (load,
// save, hydrate), project settings, validation, what one step did.
//
// This file is the module's public face: code outside packages/core/src/project/
// imports from here and nowhere else (tools/modules.mjs). What is not listed is
// the module's own business. See docs/modules/project.md.

export {
  dehydrateProjects, FORMAT_NAME, hydrateProjects, parseProjectsPayload, parseTracksPayload,
  PayloadError, projectsPayload,
} from './persistenceUtils.js'
export {
  crsChoices, dominantSystem, moveTracksToPlane, newTrackEpsg, ownLineCategory, planeGroup,
  planeInSystem, previewTracksToPlane, projectSystem, projectSystemSetting, systemChoices,
  systemDeviates, withImportSystem, withLineCategory, zoneChoices,
} from './projectSettings.js'
export { describeStep, stepTrackIds } from './stepLabel.js'
export { newFindings, validateProject } from './validateProject.js'
