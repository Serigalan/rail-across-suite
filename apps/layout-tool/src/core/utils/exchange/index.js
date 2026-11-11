// exchange — files in and out: RailJSON/OSRD and the alignment exchange file,
// Verm.ESN, ProVI, Gleislage CSV, the DB ASCII interface (MDB).
//
// This file is the module's public face: code outside src/core/utils/exchange/
// imports from here and nowhere else (tools/modules.mjs). What is not listed is
// the module's own business. See docs/modules/exchange.md.

export { exportExchange, FORMAT_VERSION } from './exchangeExport'
export {
  buildTracksFromCsv, CSV_EPSG, listLines, parseTrackPositionCsv, parseCantCsv,
} from './trackPositionCsvImport'
export { listMdbLines, parseMdbPayload } from './mdbImport'
export { ALL_LINES, runMdbImport } from './mdbPipeline'
export { placeMdbSwitches } from './mdbSwitchPlacement'
export { exportToOsrd, OSRD_URL } from './osrdExport'
export { parseOsrdRailJson } from './osrdImport'
export {
  buildProviTracks, listProviAxes, PROVI_FRAMES, proviHeights, proviPlaceableUnits, proviProject,
  proviSwitchCandidates, readProviArchive,
} from './proviImport'
export { planRailJsonImport } from './railJsonImport'
export {
  MIN_RANGE, SPEED_STEPS, SVG_SCALE, averageSpeed, kmAlong, runOf, speedProfile, speedRanges, speedRangesCsv, speedRangesSvg, speedRangesSvgSize, steppedProfile, withKm,
} from './speedRanges'
export { exportVermEsn } from './vermEsnExport'
export {
  batchReferences, buildVermEsnBatch, gradientHeightCode, gradientReferences, kmLineEntries, kmLineNumber,
  parseGradient, readAxis, readKmLines, readVermEsnArchive, referencedHeights,
} from './vermEsnImport'
