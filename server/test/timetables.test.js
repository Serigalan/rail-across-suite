import { afterEach, describe, expect, it } from 'vitest'
import { setup, signIn } from './helpers.js'

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

  it('come in from a file — all of them or none —, with a project imported or into one there is', async () => {
    ctx = await setup()
    const max = await signIn(ctx.app, 'max')
    const ada = await signIn(ctx.app, 'ada')
    const file = [{ name: 'Takt', data: { trains: [train()] } }, { name: 'Nacht', data: { trains: [] } }]

    const made = (await max('POST', '/api/projects', { title: 'Strecke', timetables: file })).json()
    expect(made.timetables).toBe(2)
    expect((await max('GET', `/api/projects/${made.project.id}/timetables`)).json().timetables.map(t => t.name)).toEqual(['Nacht', 'Takt'])
    // a file with one timetable that is not one brings no project either
    const before = (await max('GET', '/api/projects')).json().projects.length
    const bad = await max('POST', '/api/projects', { title: 'Strecke', timetables: [...file, { name: 'F', data: { trains: [train({ departure: -1 })] } }] })
    expect(bad.json()).toEqual({ error: 'train_invalid' })
    expect((await max('GET', '/api/projects')).json().projects).toHaveLength(before)

    const into = await max('POST', `/api/projects/${made.project.id}/timetables/import`, { timetables: file })
    expect(into.statusCode).toBe(201)
    // one revision of the project's operations more: the version is its number (decision 0012)
    expect(into.json().timetables).toMatchObject([{ name: 'Takt', version: 2, data: { trains: [{ number: '16001' }] } }, { name: 'Nacht', version: 2 }])
    expect((await max('POST', `/api/projects/${made.project.id}/timetables/import`, { timetables: [{ name: ' ' }] })).json()).toEqual({ error: 'name_required' })
    expect((await max('POST', `/api/projects/${made.project.id}/timetables/import`, {})).json()).toEqual({ error: 'timetable_invalid' })
    expect((await max('GET', `/api/projects/${made.project.id}/timetables`)).json().timetables).toHaveLength(4)

    // a template's, the admin's alone — with the template, or into it
    const template = (await ada('POST', '/api/projects', { title: 'Vorlage', template: true, timetables: file })).json()
    expect(template.timetables).toBe(2)
    expect((await max('POST', `/api/projects/${template.project.id}/timetables/import`, { timetables: file })).statusCode).toBe(403)
    expect((await ada('POST', `/api/projects/${template.project.id}/timetables/import`, { timetables: file })).statusCode).toBe(201)
    expect((await max('POST', `/api/projects/${template.project.id}/copy`, {})).json().timetables).toBe(4)
    // and into no project one does not see
    const adas = (await ada('POST', '/api/projects', { title: 'Adas' })).json().project
    expect((await max('POST', `/api/projects/${adas.id}/timetables/import`, { timetables: file })).statusCode).toBe(404)
  })

  it('are kept as revisions of the project\'s operations: a save on an older one is taken where its timetable is as it was there (decision 0012)', async () => {
    ctx = await setup()
    const max = await signIn(ctx.app, 'max')
    const ada = await signIn(ctx.app, 'ada')
    const { project } = (await max('POST', '/api/projects', { title: 'Strecke' })).json()
    const a = (await max('POST', `/api/projects/${project.id}/timetables`, { name: 'A', data: { trains: [train()] } })).json().timetable
    const b = (await max('POST', `/api/projects/${project.id}/timetables`, { name: 'B' })).json().timetable
    expect([a.version, b.version]).toEqual([1, 2])
    // Ada saves A, read at 1, after B came: A is as it was at 1, so it is taken
    const saved = await ada('PUT', `/api/timetables/${a.id}`, { version: 1, name: 'A', data: { trains: [train({ number: '16005' })] } })
    expect(saved.statusCode).toBe(200)
    expect(saved.json().timetable).toMatchObject({ version: 3, createdBy: { name: 'Max Muster' }, updatedBy: { name: 'Ada Admin' } })
    // Max saves A, read at 2: it changed since, so it is refused
    expect((await max('PUT', `/api/timetables/${a.id}`, { version: 2, name: 'A', data: { trains: [] } })).statusCode).toBe(409)

    const head = (await max('GET', `/api/projects/${project.id}/operations`)).json()
    expect(head).toMatchObject({ number: 3, document: { format: 'operations', version: '1.0' } })
    expect(head.hash).toMatch(/^sha256:/)
    expect(head.document.domains.timetables.collections.timetables.map(t => t.name).sort()).toEqual(['A', 'B'])
    const { revisions } = (await max('GET', `/api/projects/${project.id}/operations/revisions`)).json()
    expect(revisions.map(r => [r.number, r.changed])).toEqual([[3, [a.id]], [2, [b.id]], [1, [a.id]]])
    const first = (await max('GET', `/api/projects/${project.id}/operations/revisions/1`)).json()
    expect(first.document.domains.timetables.collections.trains.map(t => t.number)).toEqual(['16001'])
    expect((await max('GET', `/api/projects/${project.id}/operations/revisions/9`)).statusCode).toBe(404)
  })

  it('need a session, a name and trains in their shape', async () => {
    ctx = await setup()
    const max = await signIn(ctx.app, 'max')
    const { project } = (await max('POST', '/api/projects', { title: 'Strecke' })).json()
    expect((await ctx.app.inject({ method: 'GET', url: `/api/projects/${project.id}/timetables` })).statusCode).toBe(401)
    expect((await max('POST', `/api/projects/${project.id}/timetables`, { name: ' ' })).json()).toEqual({ error: 'name_required' })
    expect((await max('POST', `/api/projects/${project.id}/timetables`, { name: 'F', data: { trains: [train({ departure: -1 })] } })).json()).toEqual({ error: 'train_invalid' })

    // What the server says of a timetable's data: null where it takes it, else its error.
    const timetableFault = async (data) => {
      const res = await max('POST', `/api/projects/${project.id}/timetables`, { name: 'F', data })
      return res.statusCode === 201 ? null : res.json().error
    }
    expect(await timetableFault({ trains: [] })).toBeNull()
    expect(await timetableFault({ trains: [train()] })).toBeNull()
    expect(await timetableFault([])).toBe('timetable_invalid')
    expect(await timetableFault({ trains: [], dispatch: { mode: 'wait', priority: 'category', categories: ['ICE', 'RB'], times: { setup: 12, approach: 1000, release: 6 } } })).toBeNull()
    expect(await timetableFault({ trains: [], dispatch: { mode: 'later' } })).toBe('dispatch_invalid')
    expect(await timetableFault({ trains: [], dispatch: { times: { setup: -1 } } })).toBe('dispatch_invalid')
    expect(await timetableFault({ trains: [train(), train()] })).toBe('train_invalid')
    expect(await timetableFault({ trains: [train({ distribution: 'fast' })] })).toBe('train_invalid')
    expect(await timetableFault({ trains: [train({ points: [point('')] })] })).toBe('points_invalid')
    expect(await timetableFault({ trains: [train({ points: [point('A', { stop: 'yes' })] })] })).toBe('points_invalid')
    expect(await timetableFault({ trains: [train({ points: [point('A', { place: 'a' }), point('B', { place: 7 })] })] })).toBe('points_invalid')
    expect(await timetableFault({ trains: [train({ points: [point('A', { place: '3f2a' })] })] })).toBeNull()
    // a train not done yet is kept: a night train past midnight, no points yet
    expect(await timetableFault({ trains: [train({ departure: 25 * 3600, points: [] })] })).toBeNull()
  })
})
