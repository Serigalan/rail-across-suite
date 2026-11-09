// survey — measured axes: axis points traced in point clouds, reference axes
// (Bestandsachse), shift values against them, and the alignment fit request.
// Computed by the core's Rust module (decision 0020, step 10: crates/survey),
// called with the arguments as the JavaScript took them.
//
// This file is the module's public face: code outside packages/core/src/survey/
// imports from here and nowhere else (tools/modules.mjs). See docs/modules/survey.md.

import { viaRust } from '../native/index.js'

// ── the alignment fit ──

/**
 * The settings of the alignment fit: the chord and window of the curvature
 * diagram, those of the straight search and the tolerance of the gradient —
 * lengths in metres, the small ones in millimetres.
 */
export const ALIGN_DEFAULTS = Object.freeze({
  chord: 10, window: 6, toleranceMm: 10, step: 0.5, spacing: 6, sagittaMm: 3, minLength: 20, heightToleranceMm: 20,
})

/** The axis points of a survey as the fit reads them: the top of the lower rail as height, the cant in mm. */
export const surveyAxis = viaRust('survey.surveyAxis')
/** An axis point file (CSV of the trace, or station/E/N/H) as points; throws where it is none. */
export const parseAxisPointFile = viaRust('survey.parseAxisPointFile')
/** The request of the alignment fit for `points`, `{ straights, settings }` optional. */
export const alignRequest = viaRust('survey.alignRequest')
/** The curvature diagram of an answer: measured and fitted. */
export const fittedCurvature = viaRust('survey.fittedCurvature')
/** The elements of an answer with their deviations, against `tolerance`. */
export const elementReport = viaRust('survey.elementReport')
/** The gradient of an answer, its stretches and their deviations. */
export const gradientReport = viaRust('survey.gradientReport')
const trackFromFitRust = viaRust('survey.trackFromFit')
/**
 * A new track from an answer, `{ name, epsg, newId }`: null without elements.
 * `newId` (a function, which the module cannot be handed) names it where given.
 */
export function trackFromFit(answer, { newId, ...options } = {}) {
  const track = trackFromFitRust(answer, options)
  return track && newId ? { ...track, id: newId() } : track
}
/** Straights in the answer's form ({ from, to }) as the request takes them. */
export const straightRanges = viaRust('survey.straightRanges')
/** The straights with the `side` ('from'/'to') of straight `i` moved to `at`. */
export const moveStraightEnd = viaRust('survey.moveStraightEnd')
/** The straights with one from `a` to `b` added, in order. */
export const insertStraight = viaRust('survey.insertStraight')
/** The straights without straight `i`. */
export const deleteStraight = viaRust('survey.deleteStraight')

// ── axis surveys ──

/** A survey (domain record) from a rail trace's result. */
export const surveyFromTrace = viaRust('survey.surveyFromTrace')
/** The points of a survey, decoded: { station, easting, northing, zLeft, zRight, cant, gauge, quality }. */
export const surveyPoints = viaRust('survey.surveyPoints')
/** The survey without the points at `indices`. */
export const withoutPoints = viaRust('survey.withoutPoints')
/** Count, length and the share of good points of a survey. */
export const surveyStats = viaRust('survey.surveyStats')
/** What is wrong with a survey record, or null. */
export const surveyDefect = viaRust('survey.surveyDefect')

// ── reference axes ──

/** The longest stretch read into a project [m] (Entscheidung 195). */
export const REFERENCE_MAX_LENGTH = 2000
/** A long range cut into stretches of at most `max` metres. */
export const referenceStretches = viaRust('survey.referenceStretches')
/** A reference axis (domain record) sampled from elements, every centimetre. */
export const buildReferenceAxis = viaRust('survey.buildReferenceAxis')
/** The stationing a reference axis covers: { from, to }. */
export const axisRange = viaRust('survey.axisRange')
/** Every `every`-th point of a reference axis, [e, n] in its plane. */
export const axisOutline = viaRust('survey.axisOutline')
/** What is wrong with a reference axis record, or null. */
export const referenceAxisDefect = viaRust('survey.referenceAxisDefect')

// ── shift values ──

/** A line of elements as shift values compare it, `{ heights, station0 }` optional. */
export const comparedLine = viaRust('survey.comparedLine')
/** Whether an axis's heights can be compared with heights in `heightEpsg`. */
export const heightsComparable = viaRust('survey.heightsComparable')
/** The shift values of a line against an axis: rows every `every` metres. */
export const shiftValues = viaRust('survey.shiftValues')
/** The runs of the lifts along the line, for the elevation profile. */
export const referenceProfile = viaRust('survey.referenceProfile')
/** The lift at station `s` of the runs, or null. */
export const deviationAt = viaRust('survey.deviationAt')
/** The largest shifts and how many rows exceed the limits. */
export const shiftSummary = viaRust('survey.shiftSummary')
/**
 * Of `axes` in the line's plane, the one it runs along the furthest — the
 * object of `axes` itself, not a copy — or null.
 */
export const nearestAxis = viaRust('survey.nearestAxis', (axis, [axes]) => (axis ? (axes ?? []).find(a => a.id === axis.id) ?? axis : null))
/** The rows as a CSV file's text. */
export const shiftValuesCsv = viaRust('survey.shiftValuesCsv')
