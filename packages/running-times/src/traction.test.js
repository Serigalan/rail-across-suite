import { describe, expect, it } from 'vitest'
import { electrifiedAlong, tractionFor, withoutWire } from './traction.js'

// Two tracks run one after the other, T1 up 0…1000, T2 down 600…0.
const path = [{ trackId: 'T1', start: 0, end: 1000 }, { trackId: 'T2', start: 600, end: 0 }]
const range = (trackId, startStation, endStation, system = '15000V') => ({ id: `${trackId}-${startStation}`, at: { anchor: 'station', trackId, startStation, endStation }, system })
const record = (...ranges) => ({ electrification: { fields: {}, collections: { electrifications: ranges } } })
const mode = (isElectric) => ({ is_electric: isElectric, default_curve: { speeds: [0, 10], max_efforts: [1, 1] }, curves: [] })
const hybrid = { effort_curves: { default_mode: 'thermal', modes: { thermal: mode(false), '15000V': mode(true) } } }
const electricOnly = { effort_curves: { default_mode: '25000V', modes: { '25000V': mode(true), '15000V': mode(true) } } }

describe('the traction along a way (decision 0025)', () => {
  it('lays the ranges under wire along the path, in running order', () => {
    expect(electrifiedAlong(record(range('T1', 200, 1000), range('T2', 0, 300, '25000V')), path)).toEqual([
      { begin: 200, end: 1000, system: '15000V' }, { begin: 1300, end: 1600, system: '25000V' },
    ])
    expect(withoutWire(electrifiedAlong(record(range('T1', 200, 1000)), path), '15000V', 1600)).toEqual([{ begin: 0, end: 200 }, { begin: 1000, end: 1600 }])
  })

  it('runs the electric mode that has wire the whole way, else the thermal one, and says where an asked mode has none', () => {
    const whole = electrifiedAlong(record(range('T1', 0, 1000), range('T2', 0, 600)), path)
    expect(tractionFor(hybrid, null, whole, 1600)).toEqual({ mode: '15000V', auto: true, unpowered: [] })
    const part = electrifiedAlong(record(range('T1', 0, 1000)), path)
    expect(tractionFor(hybrid, null, part, 1600)).toMatchObject({ mode: 'thermal', unpowered: [] })
    expect(tractionFor(hybrid, '15000V', part, 1600)).toEqual({ mode: '15000V', auto: false, unpowered: [{ begin: 1000, end: 1600 }] })
    // An electric vehicle without a thermal mode keeps its own and is told where its wire is missing.
    expect(tractionFor(electricOnly, null, part, 1600)).toEqual({ mode: '25000V', auto: true, unpowered: [{ begin: 0, end: 1600 }] })
    // A net whose electrification is not planned: as always, the default mode.
    expect(tractionFor(hybrid, null, [], 1600, false)).toEqual({ mode: 'thermal', auto: false, unpowered: [] })
  })
})
