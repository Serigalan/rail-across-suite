import { afterEach, describe, expect, it } from 'vitest'
import { createHash } from 'node:crypto'
import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { setup, signIn } from './helpers.js'
import { makeLas } from '@rail-across/pointcloud/test/pointCloudFixture.js'
import { gaugeProfile, gaugeProfileRing, gaugeProfileAreas } from '@rail-across/core/crossSection'

let ctx, root
afterEach(async () => {
  await ctx?.app.close()
  ctx?.db.close()
  if (root) rmSync(root, { recursive: true, force: true })
  ctx = root = null
})

// A straight track due north over 30 m: ballast 0.17 m under the top of rail
// at 100 m, two 54E4 heads with their inner flanks, and at stations 15–16 m a
// post 1 m right of the axis reaching into the clearance outline.
const E0 = 4470000, N0 = 5332000
const HALF = 1.435 / 2
function trackCloud() {
  const pts = []
  for (let s = 0; s <= 30; s += 0.05) {
    for (let y = -2.5; y <= 2.5; y += 0.05) pts.push({ x: E0 + y, y: N0 + s, z: 99.83 })
    for (const side of [-1, 1]) {
      const inner = side * HALF
      for (let d = 0; d <= 0.067; d += 0.005) pts.push({ x: E0 + inner + side * d, y: N0 + s, z: 100 })
      for (let dz = 0.005; dz <= 0.05; dz += 0.005) pts.push({ x: E0 + inner, y: N0 + s, z: 100 - dz })
    }
    if (s >= 15 && s <= 16) for (let z = 100.5; z <= 102; z += 0.05) pts.push({ x: E0 + 1, y: N0 + s, z })
  }
  return makeLas(pts, { offset: [E0, N0, 0] })
}

const TRACK = {
  id: 't1', name: 'Gleis 1', epsg: 5678,
  elements: [{ elementType: 'straight', startNode: [E0, N0], endNode: [E0, N0 + 30], bearing: 0, length: 30, cant: 0 }],
  heights: [{ station: 0, z: 100 }, { station: 30, z: 100 }],
}
const profile = gaugeProfile('hauptgleis')
const clearance = () => ({
  kind: 'clearance', subject: { trackId: 't1', name: 'Gleis 1' },
  params: {
    track: TRACK, tracks: [TRACK], switches: [],
    ring: gaugeProfileRing(profile.points), areas: gaugeProfileAreas(profile.einragungen),
  },
})

async function start() {
  root = mkdtempSync(join(tmpdir(), 'olt-runs-'))
  ctx = await setup({ cloudRoot: root })
  const max = await signIn(ctx.app, 'max')
  const ada = await signIn(ctx.app, 'ada')
  const { project } = (await ada('POST', '/api/projects', { title: 'P' })).json()
  const maxId = (await ada('GET', '/api/admin/users')).json().users.find(u => u.login === 'max').id
  return { max, ada, project, maxId }
}

/** Upload the track's cloud and prepare it, as rail-across-cloudjobs would. */
async function readyCloud(ada, project) {
  const bytes = trackCloud()
  const { cloud } = (await ada('POST', `/api/projects/${project.id}/clouds`, {
    name: 'Gleis', fileName: 'gleis.las', fileSize: bytes.length, format: 'las', crs: 5678, heightEpsg: 7837,
    pointCount: 100000,
  })).json()
  const url = `/api/projects/${project.id}/clouds/${cloud.id}`
  const step = 4 * 1024 * 1024
  for (let at = 0; at < bytes.length; at += step) {
    const part = bytes.subarray(at, at + step)
    const res = await ctx.app.inject({
      method: 'PUT', url: `${url}/raw?offset=${at}`, cookies: { ra_session: ada.token },
      headers: {
        'content-type': 'application/octet-stream', 'x-olt-upload': '1',
        'x-olt-sha256': createHash('sha256').update(part).digest('hex'),
      },
      payload: Buffer.from(part),
    })
    expect(res.statusCode, res.body).toBe(200)
  }
  await ada('POST', `${url}/complete`)
  await ctx.app.runJobs()
  expect(ctx.db.prepare('SELECT status FROM point_cloud WHERE id = ?').pluck().get(cloud.id)).toBe('ready')
  return cloud
}

/** A test route of the server, as the admin of the test. */
const test = async (url, payload = {}) => (await ctx.app.inject({ method: 'POST', url, payload })).json()
/** A run's row as the server keeps it, or undefined. */
const runRow = (id) => ctx.db.prepare('SELECT * FROM cloud_run WHERE id = ?').get(id)

describe('long runs on the server (AP 13.7)', () => {
  it('checks the clearance along a track on the project\'s clouds and keeps what it found', async () => {
    const { ada, project } = await start()
    const cloud = await readyCloud(ada, project)
    const res = await ada('POST', `/api/projects/${project.id}/runs`, clearance())
    expect(res.statusCode, res.body).toBe(201)
    const { run } = res.json()
    expect(run).toMatchObject({ kind: 'clearance', status: 'queued', clouds: [cloud.id], subject: { trackId: 't1' } })

    expect(await ctx.app.runJobs('runs')).toBe(1)
    const done = (await ada('GET', `/api/projects/${project.id}/runs/${run.id}`)).json().run
    expect(done).toMatchObject({ status: 'done', progress: 1 })
    expect(done.result.checked).toBe(60)
    expect(done.result.stretches).toHaveLength(1)
    const [s] = done.result.stretches
    expect(s.from).toBeGreaterThan(14.5)
    expect(s.to).toBeLessThan(16.5)
    expect(s.depth).toBeGreaterThan(300)
    // The list leaves the result out.
    const list = (await ada('GET', `/api/projects/${project.id}/runs`)).json()
    expect(list.runs[0]).toMatchObject({ id: run.id, status: 'done' })
    expect(list.runs[0].result).toBeUndefined()
  })

  it('traces the rail heads along a line drawn beside the track', async () => {
    const { ada, project } = await start()
    await readyCloud(ada, project)
    const { run } = (await ada('POST', `/api/projects/${project.id}/runs`, {
      kind: 'trace', subject: { name: 'Linie' },
      params: { rail: '54E4', guide: { kind: 'line', epsg: 5678, vertices: [[E0 + 0.3, N0 + 1], [E0 + 0.3, N0 + 29]] } },
    })).json()
    expect(await ctx.app.runJobs('runs')).toBe(1)
    const done = (await ada('GET', `/api/projects/${project.id}/runs/${run.id}`)).json().run
    expect(done.status, done.error).toBe('done')
    const { points, epsg } = done.result
    expect(epsg).toBe(5678)
    expect(points.length).toBeGreaterThan(50)
    for (const p of points) expect(Math.abs(p.easting - E0)).toBeLessThan(0.01)
    expect(Math.abs(points[10].zLeft - 100)).toBeLessThan(0.01)
  })

  it('is started and cancelled only with the right; members see the runs', async () => {
    const { ada, max, maxId, project } = await start()
    await readyCloud(ada, project)
    await ada('POST', `/api/projects/${project.id}/members`, { userId: maxId })
    expect((await max('POST', `/api/projects/${project.id}/runs`, clearance())).json()).toMatchObject({ error: 'clouds_not_allowed' })
    const { run } = (await ada('POST', `/api/projects/${project.id}/runs`, clearance())).json()
    expect((await max('GET', `/api/projects/${project.id}/runs`)).json()).toMatchObject({ mayEdit: false, runs: [{ id: run.id }] })
    expect((await max('DELETE', `/api/projects/${project.id}/runs/${run.id}`)).statusCode).toBe(403)
    expect((await ada('POST', `/api/projects/${project.id}/runs`, { kind: 'clearance', params: { track: {} } })).statusCode).toBe(422)
  })

  it('without a cloud ready on the server there is nothing to run on', async () => {
    const { ada, project } = await start()
    expect((await ada('POST', `/api/projects/${project.id}/runs`, clearance())).json()).toMatchObject({ error: 'run_no_clouds' })
  })

  it('a cancelled run stops at its next step and stays cancelled', async () => {
    const { ada, project } = await start()
    await readyCloud(ada, project)
    const { run } = (await ada('POST', `/api/projects/${project.id}/runs`, clearance())).json()
    expect((await test('/api/__test/runs/claim')).run.id).toBe(run.id)
    expect((await ada('DELETE', `/api/projects/${project.id}/runs/${run.id}`)).json().run.status).toBe('cancelled')
    expect(await test(`/api/__test/runs/${run.id}/execute`)).toEqual({ done: false })
    expect(runRow(run.id)).toMatchObject({ status: 'cancelled', result: null })
    // A finished run is forgotten when deleted.
    expect((await ada('DELETE', `/api/projects/${project.id}/runs/${run.id}`)).statusCode).toBe(204)
    expect(runRow(run.id)).toBeUndefined()
  })

  it('a run that was running when the service stopped is queued again; old ones are forgotten', async () => {
    const { ada, project } = await start()
    await readyCloud(ada, project)
    const { run } = (await ada('POST', `/api/projects/${project.id}/runs`, clearance())).json()
    await test('/api/__test/runs/claim')
    await test('/api/__test/requeue')
    expect(runRow(run.id).status).toBe('queued')
    // Claimed and failed; forgotten a week after it was started.
    await test('/api/__test/runs/claim')
    ctx.db.prepare("UPDATE cloud_run SET status = 'failed', error = 'x' WHERE id = ?").run(run.id)
    expect(await test('/api/__test/runs/prune')).toEqual({ pruned: 0 })
    ctx.clock.t += 8 * 24 * 3600 * 1000
    expect(await test('/api/__test/runs/prune')).toEqual({ pruned: 1 })
  })
})

describe('re-referencing on the server (AP 13.13–13.15)', () => {
  const shift = (de) => [1, 0, 0, de, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]

  it('keeps a history, one in force, shown with the cloud; an earlier one or none can be put back', async () => {
    const { ada, max, maxId, project } = await start()
    const cloud = await readyCloud(ada, project)
    const url = `/api/projects/${project.id}/clouds/${cloud.id}/transforms`
    expect((await ada('POST', url, { matrix: shift(0.3), crs: 5678 })).statusCode).toBe(201)
    const second = (await ada('POST', url, {
      matrix: shift(0.31), crs: 5678, params: { sigma0: 0.002 }, pairs: [{ id: 'P1' }], residuals: { max: 0.004 },
    })).json().transform
    expect(second).toMatchObject({ active: true, crs: 5678, params: { sigma0: 0.002, crs: 5678 }, pairs: [{ id: 'P1' }] })
    const list = (await ada('GET', `/api/projects/${project.id}/clouds`)).json().clouds
    expect(list[0].transform).toMatchObject({ id: second.id, matrix: shift(0.31), crs: 5678 })
    const history = (await ada('GET', url)).json().transforms
    expect(history.map(t => t.active)).toEqual([true, false])
    expect(history[0].createdByName).toBe('Ada Admin')

    const back = (await ada('POST', `${url}/active`, { id: history[1].id })).json().cloud
    expect(back.transform.id).toBe(history[1].id)
    expect((await ada('POST', `${url}/active`, { id: null })).json().cloud.transform).toBeNull()

    await ada('POST', `/api/projects/${project.id}/members`, { userId: maxId })
    expect((await max('GET', url)).statusCode).toBe(200)
    expect((await max('POST', url, { matrix: shift(1), crs: 5678 })).statusCode).toBe(403)
    expect((await max('POST', `${url}/active`, { id: null })).statusCode).toBe(403)
    expect((await ada('POST', url, { matrix: [1, 2], crs: 5678 })).statusCode).toBe(422)
  })

  it('a run reads the cloud where its re-referencing puts it', async () => {
    const { ada, project } = await start()
    const cloud = await readyCloud(ada, project)
    // 5 m east, the post stands 6 m right of the axis — out of the clearance.
    await ada('POST', `/api/projects/${project.id}/clouds/${cloud.id}/transforms`, { matrix: shift(5), crs: 5678 })
    const { run } = (await ada('POST', `/api/projects/${project.id}/runs`, clearance())).json()
    expect(await ctx.app.runJobs('runs')).toBe(1)
    const done = (await ada('GET', `/api/projects/${project.id}/runs/${run.id}`)).json().run
    expect(done.status, done.error).toBe('done')
    expect(done.result.stretches).toEqual([])
  })
})
