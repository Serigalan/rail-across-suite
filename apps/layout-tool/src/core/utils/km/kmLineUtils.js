// The kilometrage of a track point against a line is the core's (track/kmLines.js),
// which the Signalling-Tool reads too; the module's files keep using it from here.
import { derivedId } from '@rail-across/core/document'

export { kmForTrackPoint, kmOnRuns, planeRuns } from '@rail-across/core/track'

/**
 * The id of the km line of a line number: the same on every device, so two
 * people fetching the same line get the same object, and a UUID as the format
 * has every id (decision 0029, phase 2).
 */
export const kmLineId = (lineNumber) => derivedId(`km-${lineNumber}`)
