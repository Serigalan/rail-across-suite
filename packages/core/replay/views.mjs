// The vectors of the cross section and the topology diagram (crates/track)
// and of the project's validation (crates/project). Frozen: their JavaScript
// is gone (decision 0005; the cross section with decision 0005) — they were recorded from it and hold the crates to what it
// answered.
import { frozen } from './record.mjs'

export default function record() {
  return {
    track: { cross_section: frozen('track', 'cross_section'), topology: frozen('track', 'topology') },
    project: { validate: frozen('project', 'validate') },
  }
}
