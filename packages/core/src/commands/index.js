// commands — what the panels of the Layout-Tool do to a project: building
// tracks, laying switches and crossings, connecting, the S-curve, splicing and
// reconnecting, editing the element table's geometry, the extended switch
// connection and the optimizer's apply. Computed by the core's Rust module
// (decision 0020, step 10: crates/commands), called with the arguments as the
// JavaScript took them — each on an action of the user's, or a preview when a
// value of the form changes.
//
// What stays JavaScript: the element table's view (tableView, elementRoles) —
// the page works it out on every render of the table, from all tracks, which
// a call to the module would carry across each time —, connectRemedies, which
// asks the caller back (outcomeFor), the texts of the optimizer's errors (the
// page's translation) and the constants the forms show.
//
// This file is the module's public face: code outside packages/core/src/commands/
// imports from here and nowhere else (tools/modules.mjs). See docs/commands.md.

import { reuseFrom, viaRust } from '../native/index.js'

/** A command on the module, what it passed through untouched given back as it was. */
const command = (op) => viaRust(`commands.${op}`, reuseFrom)

export { connectRemedies } from './connectRemedies.js'
export { rolesFor, withRole } from './elementRoles.js'
export { inRun, rowOf, tableParts, tableRoles, tableRows, trackEdit, writeRows } from './tableView.js'

/** The keys of the texts the S-curve's refusals are told by, by their reason. */
export const REASON_MSG = {
  no_solution:      'scurve_no_solution',
  too_short:        'scurve_too_short',
  too_sharp:        'scurve_too_sharp',
  branch_too_sharp: 'scurve_branch_too_sharp',
  cant_mismatch:    'scurve_cant_mismatch',
  cant_over:        'scurve_cant_over',
  off_element:      'scurve_off_element',
  min_element_length: 'scurve_min_element_length',
  mid_unknowns:     'scurve_mid_unknowns',
  mid_no_solution:  'scurve_mid_no_solution',
  mid_element_short: 'scurve_mid_element_short',
  mid_jump:         'scurve_mid_jump',
}

/** How far the position slider runs either side of the clicked station [m] (Entscheidung 313). */
export const SLIDER_REACH = 50

/** The text of an optimizer error: its detail, else its code translated by `t`. */
export function optimizeErrorText(t, code, detail) {
  if (detail) return detail
  const translated = t(`optimize_err_${code}`)
  return translated === `optimize_err_${code}` ? t('optimize_err_internal') : translated
}

export const adoptCrossover = command('adoptCrossover')
export const axisPoints = command('axisPoints')
export const buildConnectCurved = command('buildConnectCurved')
export const buildConnectStraight = command('buildConnectStraight')
export const buildCrossingConnect = command('buildCrossingConnect')
export const buildCrossingOnTrack = command('buildCrossingOnTrack')
export const buildCurvedLineTrack = command('buildCurvedLineTrack')
export const buildLineTrack = command('buildLineTrack')
export const buildParallelLineTrack = command('buildParallelLineTrack')
export const buildParallelTrack = command('buildParallelTrack')
export const buildReconnect = command('buildReconnect')
export const buildSCurve = command('buildSCurve')
const buildSpliceRust = command('buildSplice')
/** The spliced track from a solution, `{ tracks, solution, speed, cant, newId }` — `newId` a function, called for its id. */
export const buildSplice = ({ newId, ...o }) => buildSpliceRust({ ...o, ...(typeof newId === 'function' ? { newId: newId() } : newId != null ? { newId } : {}) })
export const buildSwitchConnect = command('buildSwitchConnect')
export const buildSwitchOnTrack = command('buildSwitchOnTrack')
export const cantAtTrackEnd = command('cantAtTrackEnd')
export const clearanceRequest = command('clearanceRequest')
export const computeShiftBounds = command('computeShiftBounds')
export const connectionPick = command('connectionPick')
export const crossingConnectOutcome = command('crossingConnectOutcome')
export const crossingConnectSolutions = command('crossingConnectSolutions')
export const crossingOnTrackPlacement = command('crossingOnTrackPlacement')
export const crossingOnTrackSlider = command('crossingOnTrackSlider')
export const crossoverRequest = command('crossoverRequest')
export const curveCant = command('curveCant')
export const editCommit = command('editCommit')
export const editStart = command('editStart')
export const elementPoints = command('elementPoints')
export const hasStraightMid = command('hasStraightMid')
export const isUsableStem = command('isUsableStem')
export const mergeElementEdits = command('mergeElementEdits')
export const neighbourAxis = command('neighbourAxis')
export const neighbourAxisNear = command('neighbourAxisNear')
export const optimizeRequest = command('optimizeRequest')
export const optimizedTrack = command('optimizedTrack')
export const picksAtSpeed = command('picksAtSpeed')
export const planElementChange = command('planElementChange')
export const planIntermediateChange = command('planIntermediateChange')
export const preferredShift = command('preferredShift')
export const reconnectFromAnswer = command('reconnectFromAnswer')
export const reconnectPick = command('reconnectPick')
export const reconnectRange = command('reconnectRange')
export const reconnectRequest = command('reconnectRequest')
export const replacedSpan = command('replacedSpan')
export const secondPickRefusal = command('secondPickRefusal')
export const settleConnection = command('settleConnection')
export const settleLengths = command('settleLengths')
export const shiftSnaps = command('shiftSnaps')
export const spliceFromAnswer = command('spliceFromAnswer')
export const spliceKeptCant = command('spliceKeptCant')
export const splicePick = command('splicePick')
export const spliceRequest = command('spliceRequest')
export const stems = command('stems')
export const straightShifts = command('straightShifts')
export const stretchDefaults = command('stretchDefaults')
export const switchConnectChainSolutions = command('switchConnectChainSolutions')
export const switchConnectOutcome = command('switchConnectOutcome')
export const switchConnectSolutions = command('switchConnectSolutions')
export const switchConnectionOf = command('switchConnectionOf')
export const switchOnTrackPlacement = command('switchOnTrackPlacement')
export const switchOnTrackRemnant = command('switchOnTrackRemnant')
export const switchOnTrackSlider = command('switchOnTrackSlider')
export const switchOnTrackToeValid = command('switchOnTrackToeValid')
export const takeApart = command('takeApart')
export const trackMeta = command('trackMeta')
/** The project with `trackIds` shifted by (dx, dy) in their planes — the project itself where nothing moved. */
export const translateTracks = command('translateTracks')
export const waOffset = command('waOffset')
