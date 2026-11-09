// The track crate's vectors (crates/track): making, splitting, joining and
// reversing tracks, heights, switch records and ports, links, end marks,
// platforms, routes, groups, bands. Frozen: the module's JavaScript is gone
// (decision 0021, phase D) — they were recorded from it and hold the crate to
// what it answered.
import { frozen } from './record.mjs'

export default function record() {
  return { track: frozen('track', 'track') }
}
