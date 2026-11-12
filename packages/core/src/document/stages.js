import { viaNative } from './viaNative.js'

/**
 * The construction stages of a record by their days (decision 0027 on 0003):
 * the existing state, a stage for every day something is put into or taken
 * out of service (`inServiceFrom`, `inServiceTo` of a track, switch, platform
 * or end mark), the final state. The core's module works it out (crates/doc,
 * stages).
 */

/** The stages of a record as a tool offers them: [{ key, id, date?, name? }] — 'existing', each day, 'final'. */
export const stagesOf = (record) => viaNative('document.stagesOf', record ?? {})

/**
 * The record as it stands at the stage `stage` ('existing', a day 'YYYY-MM-DD',
 * 'final'): what is in service then, a switch's port whose track is not there
 * empty, the layers' objects by their lifespans. The record as it is where it
 * has no such stage.
 */
export const recordAtStage = (record, stage) => (record ? viaNative('document.recordAtStage', { record, stage }) : record)
