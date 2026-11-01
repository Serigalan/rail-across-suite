import { afterEach, describe, expect, it } from 'vitest'
import { setup, signIn } from './helpers.js'
import { vehicleFault } from '../src/routes/vehicles.js'

let ctx
afterEach(async () => {
  await ctx?.app.close()
  ctx?.db.close()
  ctx = null
})

const curve = { speeds: [0, 10, 40], max_efforts: [200000, 200000, 50000] }
const vehicle = (patch = {}) => ({
  railjson_version: '3.4', name: 'Triebzug', length: 70, mass: 120000, max_speed: 44.4,
  startup_time: 10, startup_acceleration: 0.05, comfort_acceleration: 0.25, const_gamma: 0.6, inertia_coefficient: 1.08,
  rolling_resistance: { type: 'davis', A: 1500, B: 30, C: 5.5 },
  effort_curves: { default_mode: '15000V', modes: { '15000V': { is_electric: true, default_curve: curve, curves: [] } } },
  loading_gauge: 'G1', primary_category: 'REGIONAL_TRAIN', other_categories: [],
  ...patch,
})

describe('vehicles', () => {
  it('are everyone\'s to see and add, their creator\'s and the admins\' to change', async () => {
    ctx = await setup()
    const max = await signIn(ctx.app, 'max')
    const ada = await signIn(ctx.app, 'ada')
    const made = await max('POST', '/api/vehicles', { data: vehicle() })
    expect(made.statusCode).toBe(201)
    const { id, version } = made.json().vehicle
    expect(version).toBe(1)

    const seen = (await ada('GET', '/api/vehicles')).json().vehicles
    expect(seen).toHaveLength(1)
    expect(seen[0]).toMatchObject({ id, createdBy: { name: 'Max Muster' }, mayEdit: true, data: { name: 'Triebzug', mass: 120000 } })

    const changed = await max('PUT', `/api/vehicles/${id}`, { version: 1, data: vehicle({ name: 'Triebzug 2' }) })
    expect(changed.statusCode).toBe(200)
    expect(changed.json().vehicle).toMatchObject({ version: 2, data: { name: 'Triebzug 2' } })

    // a save on the version before is refused, with the vehicle as it is
    const stale = await ada('PUT', `/api/vehicles/${id}`, { version: 1, data: vehicle({ name: 'Alt' }) })
    expect(stale.statusCode).toBe(409)
    expect(stale.json()).toMatchObject({ error: 'stale_version', vehicle: { version: 2, data: { name: 'Triebzug 2' } } })

    expect((await ada('DELETE', `/api/vehicles/${id}`)).statusCode).toBe(204)
    expect((await max('GET', '/api/vehicles')).json().vehicles).toHaveLength(0)
    expect((await max('GET', `/api/vehicles/${id}`)).statusCode).toBe(404)
  })

  it('of someone else are only copied', async () => {
    ctx = await setup()
    const max = await signIn(ctx.app, 'max')
    const ada = await signIn(ctx.app, 'ada')
    const { id } = (await ada('POST', '/api/vehicles', { data: vehicle() })).json().vehicle
    expect((await max('GET', `/api/vehicles/${id}`)).json().vehicle.mayEdit).toBe(false)
    expect((await max('PUT', `/api/vehicles/${id}`, { version: 1, data: vehicle() })).statusCode).toBe(403)
    expect((await max('DELETE', `/api/vehicles/${id}`)).statusCode).toBe(403)
  })

  it('need a session and what the simulation needs', async () => {
    ctx = await setup()
    expect((await ctx.app.inject({ method: 'GET', url: '/api/vehicles' })).statusCode).toBe(401)
    const max = await signIn(ctx.app, 'max')
    const bad = await max('POST', '/api/vehicles', { data: vehicle({ mass: 0 }) })
    expect(bad.statusCode).toBe(422)
    expect(bad.json().error).toBe('mass_invalid')
  })

  it('are checked for their curves', () => {
    expect(vehicleFault(vehicle())).toBeNull()
    expect(vehicleFault(vehicle({ name: ' ' }))).toBe('name_required')
    expect(vehicleFault(vehicle({ inertia_coefficient: 0.9 }))).toBe('inertia_coefficient_invalid')
    expect(vehicleFault(vehicle({ rolling_resistance: { A: -1, B: 0, C: 0 } }))).toBe('rolling_resistance_invalid')
    const modes = (c) => ({ effort_curves: { default_mode: 'm', modes: { m: { is_electric: false, default_curve: c, curves: [] } } } })
    expect(vehicleFault(vehicle(modes({ speeds: [0], max_efforts: [1] })))).toBe('effort_curves_invalid')
    expect(vehicleFault(vehicle(modes({ speeds: [0, 5, 5], max_efforts: [1, 1, 1] })))).toBe('effort_curves_invalid')
    expect(vehicleFault(vehicle(modes({ speeds: [0, 5], max_efforts: [1] })))).toBe('effort_curves_invalid')
    expect(vehicleFault(vehicle({ effort_curves: { default_mode: 'x', modes: { m: { default_curve: curve, curves: [] } } } }))).toBe('effort_curves_invalid')
  })
})
