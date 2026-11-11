# Reference cases

Stretches made up for the suite — no real data — whose blocking times, headways and occupancy are worked out by hand,
step by step, apart from the code; a test holds the code to each ([roadmap](../../ROADMAP.md), phase 4, K0 and K1).

| Case | What it holds the code to | Test |
|---|---|---|
| [A — a two-track line with Ks block signals](a-two-track-line.md) | the blocking time of each block after Ril 405: setting, approach from the announcing signal, running, clearing, release; a route's own times and the interlocking's | `packages/timetables/src/reference.test.js` |

Planned: B — a single-track line with a crossing station; C — a station with an overtaking track (both for K1's
headways and compressed occupancy).
