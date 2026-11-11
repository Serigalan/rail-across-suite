import { describe, it, expect } from 'vitest'
import { switchOnTrackPlacement, switchOnTrackRemnant, switchOnTrackSlider, switchOnTrackToeValid } from './index.js'
import { viaRust } from '../native/index.js'
import { arcFrom, straightFrom } from '../geometry/elementFactory.js'
import { ALL_SWITCH_TYPES, SWITCH_TYPES, switchStraightLength } from '../switch/index.js'

// Steps of the commands, as the module runs them (not part of the face).
const switchOnTrackRouteEnd = viaRust('commands.switchOnTrackRouteEnd')
const trackOf = viaRust('commands.trackOf')

const EPSG = 25832
const START = { easting: 500000, northing: 5600000, zone: EPSG }
const FORM = SWITCH_TYPES.find(f => f.label === '760 – 1:14')

/**
 * A track of two 100 m elements at design speed `speed`: a straight and a
 * flat arc (R 20 000) — of different curvature, so a piece of either ends at
 * the node between them. Two straights would be one straight to LP.EL.01
 * (Entscheidung 363).
 */
function twoStraights(speed) {
  const a = straightFrom(START, 30, 100)
  const b = arcFrom({ easting: a.endNode[0], northing: a.endNode[1], zone: EPSG }, 30, 100, 20000)
  return trackOf([{ ...a, speed }, { ...b, speed }], EPSG, { name: 'line.001' }, 'host')
}
/** The dialog's settings; `speed` is the diverging track's (l_min 15 m at 100 km/h). */
const argsOf = (track, reversed = false, speed = 100) => ({
  track, reversed, sw: FORM, side: 'left', speed, straightLen: switchStraightLength(FORM),
})

describe('switch on track — the piece before the toe (LP.EL.01)', () => {
  const track = twoStraights(100)

  it('is none where the toe sits on a node, else the piece of the element behind it', () => {
    expect(switchOnTrackRemnant(track, 100, false, 100)).toMatchObject({ length: 0, short: false })
    expect(switchOnTrackRemnant(track, 100, true, 100)).toMatchObject({ length: 0, short: false })
    expect(switchOnTrackRemnant(track, 110, false, 100)).toMatchObject({ elIdx: 1, length: 10, lMin: 15, short: true })
    expect(switchOnTrackRemnant(track, 120, false, 100)).toMatchObject({ elIdx: 1, length: 20, short: false })
  })

  it('opening against the track, the piece behind the toe is the one towards the element end', () => {
    expect(switchOnTrackRemnant(track, 90, true, 100)).toMatchObject({ elIdx: 0, length: 10, short: true })
    expect(switchOnTrackRemnant(track, 80, true, 100)).toMatchObject({ elIdx: 0, length: 20, short: false })
    // With the track, the same toe leaves 80 m before it.
    expect(switchOnTrackRemnant(track, 80, false, 100)).toMatchObject({ length: 80, short: false })
  })

  it('is not checked where the diverging track runs below 40 km/h', () => {
    expect(switchOnTrackRemnant(track, 110, false, 30)).toMatchObject({ length: 10, lMin: null, short: false })
  })

  it('also measures the piece ahead of the toe up to the next node (Entscheidung 301)', () => {
    expect(switchOnTrackRemnant(track, 190, false, 100)).toMatchObject({ elIdx: 1, length: 90, ahead: 10, shortAhead: true })
    expect(switchOnTrackRemnant(track, 180, false, 100)).toMatchObject({ ahead: 20, shortAhead: false })
    // Opening against the track, ahead is towards the element's start.
    expect(switchOnTrackRemnant(track, 10, true, 100)).toMatchObject({ elIdx: 0, length: 90, ahead: 10, shortAhead: true })
    expect(switchOnTrackRemnant(track, 100, false, 100)).toMatchObject({ ahead: 0, shortAhead: false })
    expect(switchOnTrackRemnant(track, 190, false, 30)).toMatchObject({ ahead: 10, shortAhead: false })
  })

  it('runs on into a neighbour of the same curvature: two straights are one (Entscheidung 363)', () => {
    const a = straightFrom(START, 30, 100)
    const b = straightFrom({ easting: a.endNode[0], northing: a.endNode[1], zone: EPSG }, 30, 100)
    const line = trackOf([{ ...a, speed: 100 }, { ...b, speed: 100 }], EPSG, { name: 'line.002' }, 'line')
    // 10 m of the second straight before WA and the 100 m of the first behind them.
    expect(switchOnTrackRemnant(line, 110, false, 100)).toMatchObject({ length: 10, run: 110, short: false })
    // Ahead, 10 m to the node and the next straight beyond it.
    expect(switchOnTrackRemnant(line, 90, false, 100)).toMatchObject({ ahead: 10, runAhead: 110, shortAhead: false })
    // At WE, too: the piece beyond it is held to l_min up to the track's end.
    const L = switchStraightLength(FORM)
    const mm = (x) => Math.round(x * 1e3) / 1e3
    expect(switchOnTrackSlider(argsOf(line)).intervals.map(([a, b]) => [mm(a), mm(b)]))
      .toEqual([[15, mm(185 - L)], [mm(200 - L), mm(200 - L)]])
  })

  it('at WE, the piece of its element under the turnout and the piece left beyond it', () => {
    const L = switchStraightLength(FORM)
    // WE 10 m into the arc: 10 m under the turnout, 90 m beyond.
    expect(switchOnTrackRemnant(track, 110 - L, false, 100, L).end)
      .toMatchObject({ elIdx: 1, under: 10, beyond: 90, shortUnder: true, shortBeyond: false })
    // WE 10 m before the end of the straight: one piece under it with WA, 10 m left beyond.
    expect(switchOnTrackRemnant(track, 90 - L, false, 100, L).end)
      .toMatchObject({ elIdx: 0, under: 0, beyond: 10, shortBeyond: true })
    // WE on the element change: nothing left on either side.
    expect(switchOnTrackRemnant(track, 100 - L, false, 100, L).end).toMatchObject({ under: 0, beyond: 0, shortUnder: false, shortBeyond: false })
    // Opening against the track, WE lies behind WA: 10 m beyond it towards the straight's start.
    const back = switchOnTrackRemnant(track, 10 + L, true, 100, L).end
    expect(back).toMatchObject({ elIdx: 0, shortBeyond: true })
    expect(back.beyond).toBeCloseTo(10, 9)
    // A sliver beyond WE is never a place to stand.
    expect(switchOnTrackToeValid(argsOf(track), 100 - L + 2.5e-5)).toBe(false)
    expect(switchOnTrackToeValid(argsOf(track), 100 - L)).toBe(true)
  })

  it('takes the speed of the diverging track, not the element\'s (Entscheidung 219)', () => {
    // 10 m are short at 100 km/h, enough at 60 (l_min 6 m) — on either element.
    expect(switchOnTrackRemnant(track, 110, false, 60)).toMatchObject({ lMin: 6, short: false })
    expect(switchOnTrackRemnant(twoStraights(null), 110, false, 100)).toMatchObject({ lMin: 15, short: true })
  })
})

describe('switch on track — what the slider runs over', () => {
  const track = twoStraights(100)
  const room = 200 - switchStraightLength(FORM)

  it('the stretches where the turnout lies and keeps l_min on both sides of WA and WE (Entscheidungen 301, 309)', () => {
    const L = switchStraightLength(FORM)
    const { intervals } = switchOnTrackSlider(argsOf(track))
    // WA: 15 m from the track's begin and from the element change. WE: 15 m
    // from the element change and the track's end — or on either of them.
    const expected = [[15, 85 - L], [100 - L, 100 - L], [115 - L, 85], [100, 100], [115, 185 - L], [room, room]]
    expect(intervals).toHaveLength(expected.length)
    intervals.forEach(([a, b], i) => {
      expect(a).toBeCloseTo(expected[i][0], 2)
      expect(b).toBeCloseTo(expected[i][1], 2)
    })
    for (const [x, y] of intervals) {
      expect(switchOnTrackToeValid(argsOf(track), x)).toBe(true)
      expect(switchOnTrackToeValid(argsOf(track), y)).toBe(true)
    }
  })

  it('every stretch where the diverging track has no speed to check', () => {
    const { intervals } = switchOnTrackSlider(argsOf(track, false, 30))
    expect(intervals).toHaveLength(1)
    expect(intervals[0][0]).toBeLessThan(1)
    // Up to WE on the track's end, exactly.
    expect(intervals[0][1]).toBe(room)
  })

  it('runs 50 m either side of the clicked station (Entscheidung 313)', () => {
    const { intervals } = switchOnTrackSlider(argsOf(track), 120)
    expect(intervals[0][0]).toBeGreaterThanOrEqual(70)
    expect(intervals.at(-1)[1]).toBeLessThanOrEqual(170)
    expect(intervals.some(([a, b]) => a <= 100 && b >= 100)).toBe(true)
    // The element change itself, off the metre grid of the window, is still a place to stand.
    const off = switchOnTrackSlider(argsOf(track), 120.37).intervals
    expect(off.some(([a, b]) => a <= 100 + 1e-9 && b >= 100 - 1e-9)).toBe(true)
    // Near the track's begin the window ends there.
    expect(switchOnTrackSlider(argsOf(track), 20).intervals.at(-1)[1]).toBeLessThanOrEqual(70)
  })

  it('snaps where WA or WE fall on an element change (Entscheidung 310) — exactly there', () => {
    const L = switchStraightLength(FORM)
    const { snaps } = switchOnTrackSlider(argsOf(track))
    for (const s of [0, 100, 200, 100 - L, 200 - L]) expect(snaps).toContain(s)
    // Opening against the track, WE lies behind WA in the track's direction.
    expect(switchOnTrackSlider(argsOf(track, true)).snaps).toContain(100 + L)
  })

  it('snaps onto what the dialog asks for: WA, WE or the technical end', () => {
    const L = switchStraightLength(FORM)
    const snapsOf = (snap, sw = FORM) => switchOnTrackSlider({ ...argsOf(track), sw, straightLen: switchStraightLength(sw), snap }).snaps
    expect(snapsOf('toe')).toEqual([0, 100, 200])
    expect(snapsOf('end')).toEqual([-L, 100 - L, 200 - L])
    // Curved to its end, its technical end is WE.
    expect(snapsOf('technical')).toEqual(snapsOf('end'))
    // With a straight end piece it is where the arc ends — its length from WA.
    const form = ALL_SWITCH_TYPES.find(f => f.label === '500 – 1:14')
    const arc = form.R * Math.atan(1 / form.ratio)
    expect(snapsOf('technical', form).map(s => Math.round(s * 1e9) / 1e9)).toEqual([-arc, 100 - arc, 200 - arc].map(s => Math.round(s * 1e9) / 1e9))
  })
})

describe('switch on track — where the new route ends, without the symbol', () => {
  /** A track of a straight and an arc of `r`, 300 m each. */
  function bentHost(r) {
    const a = straightFrom(START, 30, 300)
    const b = arcFrom({ easting: a.endNode[0], northing: a.endNode[1], zone: EPSG }, 30, 300, r)
    return trackOf([a, b], EPSG, { name: 'line.001' }, 'host')
  }
  const full = (args, x) => {
    const g = switchOnTrackPlacement({ ...args, toeStation: x }).geom
    return g.swapped ? { utm: g.swapped.straightUtm, bearing: g.swapped.mainEndBearing }
      : { utm: g.curvedUtm, bearing: g.branchEndBearing }
  }

  it('is the end the full geometry gives — on a straight, across a joint, bent either way, swapped', () => {
    for (const r of [1200, -1200, -760, 760]) {
      for (const side of ['left', 'right']) {
        for (const reversed of [false, true]) {
          const args = { track: bentHost(r), reversed, sw: FORM, side, straightLen: switchStraightLength(FORM) }
          for (const x of [150, 290, 420]) {
            const light = switchOnTrackRouteEnd(args, x)
            const want = full(args, x)
            expect(Math.hypot(light.utm.easting - want.utm.easting, light.utm.northing - want.utm.northing)).toBeLessThan(1e-9)
            expect(Math.abs(((light.bearing - want.bearing + 540) % 360) - 180)).toBeLessThan(1e-9)
          }
        }
      }
    }
  })

  it('is null where the turnout cannot lie', () => {
    expect(switchOnTrackRouteEnd(argsOf(twoStraights(100)), 195)).toBeNull()
  })
})
