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
