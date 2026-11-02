import { describe, expect, it } from 'vitest'
import {
  comfortsOf, curveFor, curveFromPower, fromOsrd, modeLabel, newVehicle, problemsOf, resistanceAt, rollingStockOf, toOsrd,
  withComfort, withCurve, withMode, withoutCurve, withoutMode,
} from './vehicles.js'
import { BUILT_IN } from './builtIn.js'
import { sortedEntries, suiteEntry } from './catalogue.js'

const curve = (speeds, efforts) => ({ speeds, max_efforts: efforts })
/** A rolling stock as OSRD's example files have it: resistance per kg, two comforts, two modes. */
const osrd = () => ({
  railjson_version: '3.3', id: 7, locked: true, version: 3, name: 'EMU Regional train', mass: 260000, length: 100, max_speed: 44.4,
  startup_time: 20, startup_acceleration: 0.05, comfort_acceleration: 0.25, const_gamma: 0.5, inertia_coefficient: 1.05,
  rolling_resistance: { type: 'davis', A: 0.0088, B: 0.0002628, C: 0.000029808 },
  effort_curves: {
    default_mode: '25000V',
    modes: {
      '25000V': {
        is_electric: true, default_curve: curve([0, 20, 44.4], [200000, 200000, 90000]),
        curves: [
          { cond: { comfort: 'STANDARD', electrical_profile_level: null, power_restriction_code: null }, curve: curve([0, 20, 44.4], [200000, 200000, 90000]) },
          { cond: { comfort: 'AIR_CONDITIONING', electrical_profile_level: null, power_restriction_code: null }, curve: curve([0, 20, 44.4], [190000, 190000, 85000]) },
          { cond: { comfort: 'STANDARD', electrical_profile_level: '22500V', power_restriction_code: null }, curve: curve([0, 44.4], [150000, 70000]) },
        ],
      },
      '1500V': { is_electric: true, default_curve: curve([0, 30], [120000, 60000]), curves: [] },
    },
  },
  primary_category: 'REGIONAL_TRAIN', other_categories: [], supported_signaling_systems: [{ type: 'BAL' }], liveries: [],
})

describe('vehicles in OSRD\'s form', () => {
  it('are taken in, their resistance per kg made the whole train\'s', () => {
    const { vehicle, notes } = fromOsrd(osrd())
    expect(notes).toEqual(['resistance_per_kg'])
    expect(vehicle.rolling_resistance.A).toBeCloseTo(0.0088 * 260000, 9)   // 2288 N
    expect(vehicle.rolling_resistance.C).toBeCloseTo(7.75, 2)
    expect(vehicle.id).toBeUndefined()
    expect(vehicle.locked).toBeUndefined()
    expect(vehicle.metadata.series).toBe('')
    // what the tool does not edit goes along
    expect(vehicle.supported_signaling_systems).toEqual([{ type: 'BAL' }])
    // resistances in N stay as they are
    expect(fromOsrd({ ...osrd(), rolling_resistance: { type: 'davis', A: 2300, B: 70, C: 7.7 } }).notes).toEqual([])
    expect(() => fromOsrd({ name: 'x' })).toThrow('not_rolling_stock')
  })

  it('go back out as RailJSON', () => {
    const out = toOsrd({ ...fromOsrd(osrd()).vehicle, id: 'x' })
    expect(out.railjson_version).toBe('3.4')
    expect(out.id).toBeUndefined()
    expect(out.effort_curves.modes['1500V'].default_curve.speeds).toEqual([0, 30])
  })

  it('run with the curve of their mode and comfort, as OSRD chooses it', () => {
    const { vehicle } = fromOsrd(osrd())
    expect(curveFor(vehicle).max_efforts[0]).toBe(200000)
    expect(curveFor(vehicle, '25000V', 'AIR_CONDITIONING').max_efforts[0]).toBe(190000)
    expect(curveFor(vehicle, '25000V', 'HEATING').max_efforts[0]).toBe(200000)   // none for heating: the standard one
    expect(curveFor(vehicle, '1500V', 'AIR_CONDITIONING').max_efforts[0]).toBe(120000)   // no curves: the default
    expect(comfortsOf(vehicle, '25000V')).toEqual(['STANDARD', 'AIR_CONDITIONING'])
    const rs = rollingStockOf(vehicle, { mode: '1500V' })
    expect(rs.effort_curve).toEqual([{ speed: 0, max_effort: 120000 }, { speed: 30, max_effort: 60000 }])
    expect(rs.rolling_resistance).toEqual({ A: vehicle.rolling_resistance.A, B: vehicle.rolling_resistance.B, C: vehicle.rolling_resistance.C })
  })

  it('are checked for what the simulation needs', () => {
    expect(problemsOf(newVehicle('Triebzug'))).toEqual([])
    const v = newVehicle('')
    v.mass = 0
    v.effort_curves.modes['15000V'].curves[0].curve = curve([0, 20, 10], [1, 1, 1])
    expect(problemsOf(v).map(p => `${p.field}:${p.key}`)).toEqual(['name:name_required', 'mass:positive', 'effort_curves.15000V.0:curve_order'])
  })

  it('have their modes and comforts edited', () => {
    let v = newVehicle('Zweikraft')
    v = withMode(v, 'thermal', '15000V')
    expect(Object.keys(v.effort_curves.modes)).toEqual(['15000V', 'thermal'])
    expect(v.effort_curves.modes.thermal.is_electric).toBe(false)
    v = withComfort(v, 'thermal', 'AIR_CONDITIONING')
    expect(comfortsOf(v, 'thermal')).toEqual(['STANDARD', 'AIR_CONDITIONING'])
    // the standard curve is the default curve too
    v = withCurve(v, 'thermal', 0, curve([0, 30], [100000, 50000]))
    expect(v.effort_curves.modes.thermal.default_curve.max_efforts).toEqual([100000, 50000])
    v = withoutCurve(v, 'thermal', 1)
    v = withoutMode(v, '15000V')
    expect(v.effort_curves.default_mode).toBe('thermal')
    expect(v.raise_pantograph_time).toBeNull()
  })

  it('get a curve from their starting effort and power', () => {
    const c = curveFromPower({ effort: 200000, power: 4000000, maxSpeed: 160 / 3.6 })
    expect(c.speeds[0]).toBe(0)
    expect(c.speeds.at(-1)).toBeCloseTo(160 / 3.6, 9)
    expect(c.speeds).toContainEqual(20)   // where the power runs out
    expect(c.max_efforts.at(-1)).toBeCloseTo(4000000 / (160 / 3.6), 6)
  })

})

describe('the built-in vehicles', () => {
  it('are OSRD\'s eleven, each fit to run, their resistance made the whole train\'s', () => {
    expect(BUILT_IN).toHaveLength(11)
    for (const e of BUILT_IN) {
      expect(e.key).toMatch(/^osrd:/)
      expect(e.mayEdit).toBe(false)
      expect(problemsOf(e.vehicle)).toEqual([])
      if (resistanceAt(e.vehicle, 1) > 0) expect(e.notes).toEqual(['resistance_per_kg'])
    }
    const tgv = BUILT_IN.find(e => e.key === 'osrd:high-speed-train').vehicle
    expect(resistanceAt(tgv, 320 / 3.6) / 1000).toBeCloseTo(66.5, 0)   // kN at 320 km/h
  })

  it('come after the suite\'s in the catalogue', () => {
    const own = suiteEntry({ id: 'u1', version: 1, data: newVehicle('Zug'), mayEdit: true, createdBy: { name: 'Max' } })
    const list = sortedEntries([...BUILT_IN, own])
    expect(list[0].key).toBe('u1')
    expect(list[1].vehicle.name).toBe('Commuter train')
  })

  it('name their modes in the words of the texts', () => {
    const t = (k) => ({ mode_25000V: '25 kV 50 Hz' })[k] ?? k
    expect(modeLabel(t, '25000V')).toBe('25 kV 50 Hz')
    expect(modeLabel(t, '1000V')).toBe('1000V')
  })
})
