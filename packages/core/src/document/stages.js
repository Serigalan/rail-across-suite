import { viaNative } from './viaNative.js'

/**
 * The states a project passes through, worked out of its lifespans (decision
 * 0007): the existing one, one for every day something is put into or taken
 * out of service (a lifespan's `from`, the day after its `to` — of a track,
 * switch, platform, end mark, civil structure or an object of a layer), the
 * final one. Nothing of it is kept: there is no register of stages. The
 * core's module works it out (crates/doc, stages).
 */

/** The states of a record as a tool offers them: [{ key, date? }] — 'existing', each day, 'final'. */
export const stagesOf = (record) => viaNative('document.stagesOf', record ?? {})

/**
 * The record as it stands at the state `stage` ('existing', a day 'YYYY-MM-DD',
 * 'final'): what is in service then, a switch's port whose track is not there
 * empty, the civil structures and the layers' objects by their lifespans. On
 * a day stands what has a first day up to it and no last day before it; what
 * is new or removed with its day open ("open") only tells the final state
 * from the days. The record as it is where `stage` is none of these.
 */
export const recordAtStage = (record, stage) => (record ? viaNative('document.recordAtStage', { record, stage }) : record)
