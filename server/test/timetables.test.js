import { afterEach, describe, expect, it } from 'vitest'
import { setup, signIn } from './helpers.js'
import { timetableFault } from '../src/routes/timetables.js'

let ctx
afterEach(async () => {
  await ctx?.app.close()
  ctx?.db.close()
  ctx = null
})

const point = (station, patch = {}) => ({ id: `p-${station}`, station, track: null, stop: true, dwell: 60, departure: null, ...patch })
const train = (patch = {}) => ({
  id: 't1', number: '16001', category: 'RB', name: '', vehicle: 'osrd:emu-regional-train', mode: null, comfort: 'STANDARD',
  departure: 6 * 3600, margin: '5 %', distribution: 'mareco', points: [point('A-Dorf'), point('C-Stadt')], ...patch,
})

describe('timetables', () => {
  it('are a project\'s, seen and changed by whoever sees it', async () => {
    ctx = await setup()
    const max = await signIn(ctx.app, 'max')
    const ada = await signIn(ctx.app, 'ada')
    const { project } = (await max('POST', '/api/projects', { title: 'Strecke' })).json()
    const made = await max('POST', `/api/projects/${project.id}/timetables`, { name: 'Fahrplan 2027', data: { trains: [train()] } })
    expect(made.statusCode).toBe(201)
    const { id } = made.json().timetable
    expect(made.json().timetable).toMatchObject({ projectId: project.id, name: 'Fahrplan 2027', version: 1, mayEdit: true, data: { trains: [{ number: '16001' }] } })

    const listed = (await ada('GET', `/api/projects/${project.id}/timetables`)).json()
    expect(listed.timetables.map(t => t.id)).toEqual([id])

    const saved = await max('PUT', `/api/timetables/${id}`, { version: 1, name: 'Fahrplan 2027', data: { trains: [train(), train({ id: 't2', number: '16003' })] } })
    expect(saved.statusCode).toBe(200)
    expect(saved.json().timetable).toMatchObject({ version: 2, updatedBy: { name: 'Max Muster' } })

    // a save on the version before is refused, with the timetable as it is
    const stale = await ada('PUT', `/api/timetables/${id}`, { version: 1, name: 'Alt', data: { trains: [] } })
    expect(stale.statusCode).toBe(409)
    expect(stale.json()).toMatchObject({ error: 'stale_version', timetable: { version: 2, name: 'Fahrplan 2027' } })

    expect((await max('DELETE', `/api/timetables/${id}`)).statusCode).toBe(204)
    expect((await max('GET', `/api/projects/${project.id}/timetables`)).json().timetables).toHaveLength(0)
    expect((await max('GET', `/api/timetables/${id}`)).statusCode).toBe(404)
  })

  it('are not there for whoever does not see the project, and a template\'s are the admins\'', async () => {
    ctx = await setup()
    const max = await signIn(ctx.app, 'max')
    const ada = await signIn(ctx.app, 'ada')
    const { project } = (await ada('POST', '/api/projects', { title: 'Adas' })).json()
    const { id } = (await ada('POST', `/api/projects/${project.id}/timetables`, { name: 'F' })).json().timetable
    expect((await max('GET', `/api/projects/${project.id}/timetables`)).statusCode).toBe(404)
    expect((await max('GET', `/api/timetables/${id}`)).statusCode).toBe(404)
    expect((await max('PUT', `/api/timetables/${id}`, { version: 1, name: 'F' })).statusCode).toBe(404)

    const template = (await ada('POST', '/api/projects', { title: 'Vorlage', template: true })).json().project
    const tt = (await ada('POST', `/api/projects/${template.id}/timetables`, { name: 'Muster' })).json().timetable
    const seen = (await max('GET', `/api/projects/${template.id}/timetables`)).json()
    expect(seen).toMatchObject({ mayEdit: false, timetables: [{ id: tt.id, mayEdit: false }] })
    expect((await max('POST', `/api/projects/${template.id}/timetables`, { name: 'Meins' })).statusCode).toBe(403)
    expect((await max('DELETE', `/api/timetables/${tt.id}`)).statusCode).toBe(403)
  })

  it('need a session, a name and trains in their shape', async () => {
    ctx = await setup()
    const max = await signIn(ctx.app, 'max')
    const { project } = (await max('POST', '/api/projects', { title: 'Strecke' })).json()
    expect((await ctx.app.inject({ method: 'GET', url: `/api/projects/${project.id}/timetables` })).statusCode).toBe(401)
    expect((await max('POST', `/api/projects/${project.id}/timetables`, { name: ' ' })).json()).toEqual({ error: 'name_required' })
    expect((await max('POST', `/api/projects/${project.id}/timetables`, { name: 'F', data: { trains: [train({ departure: -1 })] } })).json()).toEqual({ error: 'train_invalid' })

    expect(timetableFault({ trains: [] })).toBeNull()
    expect(timetableFault({ trains: [train()] })).toBeNull()
    expect(timetableFault(null)).toBe('timetable_invalid')
    expect(timetableFault({ trains: [], dispatch: { mode: 'wait', priority: 'category', categories: ['ICE', 'RB'], times: { setup: 12, approach: 1000, release: 6 } } })).toBeNull()
    expect(timetableFault({ trains: [], dispatch: { mode: 'later' } })).toBe('dispatch_invalid')
    expect(timetableFault({ trains: [], dispatch: { times: { setup: -1 } } })).toBe('dispatch_invalid')
    expect(timetableFault({ trains: [train(), train()] })).toBe('train_invalid')
    expect(timetableFault({ trains: [train({ distribution: 'fast' })] })).toBe('train_invalid')
    expect(timetableFault({ trains: [train({ points: [point('')] })] })).toBe('points_invalid')
    expect(timetableFault({ trains: [train({ points: [point('A', { stop: 'yes' })] })] })).toBe('points_invalid')
    // a train not done yet is kept: a night train past midnight, no points yet
    expect(timetableFault({ trains: [train({ departure: 25 * 3600, points: [] })] })).toBeNull()
  })
})
