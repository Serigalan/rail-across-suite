# Reference case A — a two-track line with Ks block signals

One track of a two-track line, run one way, made up for the suite. Worked out by hand after Ril 405; the test
`packages/timetables/src/reference.test.js` holds `blocksAlong` and `blockingTimes` to the figures below.

## The stretch

Track `L`, 6000 m, run from station 0 to 6000 (`up`).

| Signal | At [m] | Kind (logical signal) | Sight distance | Overlap beyond it |
|---|---|---|---|---|
| vS1 | 500 | Ks distant signal | 200 m required | — |
| S1 | 1500 | Ks multi-section signal | — | — |
| S2 | 3500 | Ks multi-section signal | — | 200 m |
| S3 | 5500 | Ks multi-section signal | — | 200 m |

- Route S1 → S2: setting time 10 s, release time 5 s (its own).
- The interlocking's times (`signalling.fields.routeTimes`): setting 12 s, release 6 s — for every route that states
  none.
- The timetable's times, where the signalling says nothing: setting 12 s, release 6 s, approach 1000 m.

## The train

150 m long, running at a constant 20 m/s (72 km/h) from station 0 at 0 s, without a stop: it is at `x` metres at
`x / 20` seconds, and at the end, 6000 m, at 300 s.

## The blocks

From main signal to main signal, the first from the start, the last to the end:

| Block | From | To | Overlap to |
|---|---|---|---|
| 0 | start, 0 m | S1, 1500 m | 1500 m |
| 1 | S1, 1500 m | S2, 3500 m | 3700 m |
| 2 | S2, 3500 m | S3, 5500 m | 5700 m |
| 3 | S3, 5500 m | end, 6000 m | 6000 m |

## The blocking times, part by part

A block is held from the moment the train must find it free, less the time to set its route, until its tail has
passed the signal that ends it and the route is released (Ril 405: Fahrstraßenbildezeit, Sicht- und
Annäherungsfahrzeit, Fahrzeit im Block, Räumfahrzeit, Auflösezeit). The overlap beyond that signal is held with the
block; where the variant has track sections, each is released as the tail leaves it (not part of this case).

**Block 0** — the train starts in it: asked for at its start, 0 s; no signal begins it, so the timetable's
setting time: begins at 0 − 12 = **−12 s**. Its end S1 at 1500 m: cleared by the tail at 1500 + 150 = 1650 m,
82.5 s; no route, so the timetable's release time: ends at 82.5 + 6 = **88.5 s**.

**Block 1** — S1 is announced by the distant signal vS1 at 500 m, seen 200 m before it: asked for at 300 m,
15 s. Its route S1 → S2 sets in 10 s: begins at 15 − 10 = **5 s**. Cleared when the tail has passed S2,
3500 + 150 = 3650 m, 182.5 s; the route releases in 5 s: ends at **187.5 s**.

**Block 2** — S2 is announced by S1, a multi-section signal showing Ks 2, at 1500 m, no sight distance given:
asked for at 1500 m, 75 s. Its route states no times: the interlocking's, 12 s: begins at 75 − 12 = **63 s**.
Cleared when the tail has passed S3, 5500 + 150 = 5650 m, 282.5 s; released in the interlocking's 6 s: ends at
**288.5 s**.

**Block 3** — S3 is announced by S2 at 3500 m: asked for at 175 s; the interlocking's 12 s: begins at
**163 s**. The last block, held until the train arrives at 300 s, released in 6 s: ends at **306 s**.

| Block | Begins [s] | Ends [s] |
|---|---|---|
| 0 | −12 | 88.5 |
| 1 | 5 | 187.5 |
| 2 | 63 | 288.5 |
| 3 | 163 | 306 |

The sight distance stands for the sight time of Ril 405 here: the block is asked for where the announcing signal is
first seen. A sight time of the operation's (operations-1.0) is not part of this case.

## Headways and the compressed occupancy (K1)

A second train beside the first: **B**, as long, at a constant 10 m/s from station 0 at 0 s, arriving at 6000 m at
600 s. Its blocks by the same rules as A's:

| Block | Begins [s] | Ends [s] | worked out |
|---|---|---|---|
| 0 | −12 | 171 | asked for at its start; tail past S1 at 1650 m, 165 s, + 6 s |
| 1 | 20 | 370 | asked for at 300 m, 30 s, − 10 s; tail past S2 at 3650 m, 365 s, + 5 s |
| 2 | 138 | 571 | asked for at S1, 150 s, − 12 s; tail past S3 at 5650 m, 565 s, + 6 s |
| 3 | 338 | 606 | asked for at S2, 350 s, − 12 s; arrived at 600 s, + 6 s |

**What two blocks hold the same.** The track has no track sections, so a block is held whole — its overlap with
it. Block 1 reaches 200 m into block 2's track (S2's overlap, 3500–3700 m), block 2 as far into block 3's: each
of these holds what the other holds there. So a train behind finds its block 1 free only once the train before has
left block 2: without a track section of its own, the overlap is only known free when the whole block it lies in is.

**The minimum headway** of a train behind another (both at station 0 at their own 0 s) is the largest of the
first's end less the second's begin over every two blocks that hold the same track:

| Before → behind | Block pairs that bind | Minimum headway [s] |
|---|---|---|
| A → A | A's block 2 (ends 288.5) and the next A's block 1 (begins 5) | **283.5** |
| A → B | A's block 2 (288.5) and B's block 1 (20) | **268.5** |
| B → A | B's block 2 (571) and A's block 1 (5) | **566** |
| B → B | B's block 2 (571) and the next B's block 1 (20) | **551** |

(Same block with same block alone, as a line without overlaps would have it, A → A would be 225.5 s, block 2's
288.5 − 63.)

**The timetable**, one hour from 0 s: A1 at 0 s, B1 at 300 s, A2 at 1200 s, A3 at 1800 s. Compressed after UIC
Code 406 — in this order, each as early behind those before as their blocking times let it, never before the one
before it:

| Train | Runs at [s] | Compressed to [s] | Bound by | Its blocking times end at [s] |
|---|---|---|---|---|
| A1 | 0 | 0 | — | 306 |
| B1 | 300 | 268.5 | A1 (A → B 268.5) | 874.5 |
| A2 | 1200 | 834.5 | B1 (268.5 + 566) | 1140.5 |
| A3 | 1800 | 1118 | A2 (834.5 + 283.5) | 1424 |

**The occupancy time** runs from the first blocking time begun, A1's block 0 at −12 s, to the last ended, A3's
block 3 at 1424 s: **1436 s**, an occupancy rate of 1436 / 3600 = **39.9 %** of the hour. The trains that set it,
each bound to the one before: **A1, B1, A2, A3**. The test `crates/capacity/tests/reference.rs` holds the
capacity crate to these figures.
