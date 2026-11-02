// commands — what the dialogs commit, as pure functions of their inputs: new
// tracks and elements, switches, connecting, splicing, reconnecting, geometry
// edits and what an optimizer run sends and keeps.
//
// This file is the module's public face: code outside packages/core/src/commands/
// imports from here and nowhere else (tools/modules.mjs). What is not listed is
// the module's own business. See docs/modules/commands.md.

export { buildCrossingConnect, crossingConnectOutcome, crossingConnectSolutions } from './connectCrossing.js'
export { connectRemedies } from './connectRemedies.js'
export { adoptCrossover, crossoverRequest } from './crossover.js'
export { editCommit, editStart, switchConnectionOf, takeApart } from './crossoverEdit.js'
export { buildSwitchConnect, switchConnectChainSolutions, switchConnectOutcome, switchConnectSolutions } from './connectSwitch.js'
export { rolesFor, withRole } from './elementRoles.js'
export { inRun, rowOf, tableParts, tableRoles, tableRows, trackEdit, writeRows } from './tableView.js'
export { mergeElementEdits, planElementChange, planIntermediateChange } from './editGeometry.js'
export { optimizedTrack, optimizeErrorText, optimizeRequest } from './optimizeApply.js'
export {
  axisPoints, buildReconnect, elementPoints, reconnectFromAnswer, reconnectPick, reconnectRange,
  reconnectRequest, replacedSpan, stretchDefaults,
} from './reconnect.js'
export {
  buildSCurve, computeShiftBounds, connectionPick, isUsableStem, preferredShift, REASON_MSG,
  settleConnection, shiftSnaps, stems, waOffset,
} from './sCurve.js'
export {
  buildSplice, clearanceRequest, neighbourAxis, neighbourAxisNear, secondPickRefusal,
  picksAtSpeed, settleLengths, spliceFromAnswer, spliceKeptCant, splicePick, spliceRequest,
} from './splice.js'
export {
  buildCrossingOnTrack, buildSwitchOnTrack, crossingOnTrackPlacement, crossingOnTrackSlider,
  SLIDER_REACH, switchOnTrackPlacement, switchOnTrackRemnant, switchOnTrackSlider,
  switchOnTrackToeValid,
} from './switches.js'
export {
  buildConnectCurved, buildConnectStraight, buildCurvedLineTrack, buildLineTrack,
  buildParallelLineTrack, buildParallelTrack, cantAtTrackEnd, curveCant, trackMeta,
} from './tracks.js'
