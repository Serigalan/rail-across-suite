import { afterEach, describe, expect, it } from 'vitest'
import { gunzipSync } from 'node:zlib'
import { openDatabase } from '../src/db.js'
import { createStore } from '../src/store.js'
import { runDataMigrations, SYSTEM_LOGIN } from '../src/dataMigrations.js'

let db
afterEach(() => { db?.close(); db = null })

const straight = (a, b) => ({
  elementType: 0, startNode: a, endNode: b, length: Math.hypot(b[0] - a[0], b[1] - a[1]),
  bearing: (Math.atan2(b[0] - a[0], b[1] - a[1]) * 180 / Math.PI + 360) % 360,
})

function setUp(elements) {
  db = openDatabase(':memory:')
  db.prepare(`INSERT INTO user (login, name, password_hash, role, created_at) VALUES ('anna', 'Anna', 'x', 'admin', '2026-01-01')`).run()
  const store = createStore(db)
  const made = store.createProject({
    title: 'Halle', payload: { id: 'p1', tracks: [{ id: 't1', name: 'T1', epsg: 25832, elements }], switches: [], platforms: [] },
    authorId: 1, errorKeys: [],
  })
  return { store, made }
}

describe('the data steps', () => {
  it('closes a millimetre joint as a revision of its own, by the server, once', () => {
    const { store, made } = setUp([straight([500000, 5600000], [500100, 5600000]), straight([500100.0005, 5600000], [500200, 5600000])])
    const ran = runDataMigrations(db, store)
    expect(ran.map(r => r.name)).toEqual(['0020-node-tolerance-1um'])
    expect(ran[0].note).toBe('1 Revisionen, 1 Stöße')
    const [variant] = store.variants(made.projectId)
    const head = store.revision(variant.head_revision_id)
    expect(head.meta.number).toBe(2)
    expect(head.meta.message).toMatch(/1 Stöße bis 1 mm geschlossen/)
    const system = db.prepare('SELECT id, active FROM user WHERE login = ?').get(SYSTEM_LOGIN)
    expect(head.meta.author.id).toBe(system.id)
    expect(system.active).toBe(0)
    const doc = JSON.parse(gunzipSync(db.prepare('SELECT payload FROM revision WHERE id = ?').pluck().get(head.meta.id)).toString())
    expect(JSON.stringify(doc)).toContain('500100')
    expect(runDataMigrations(db, store)).toEqual([])
  })

  it('writes nothing, and makes no user, where every joint is closed', () => {
    const { store } = setUp([straight([500000, 5600000], [500100, 5600000]), straight([500100, 5600000], [500200, 5600000])])
    expect(runDataMigrations(db, store)[0].note).toBe('0 Revisionen, 0 Stöße')
    expect(db.prepare('SELECT COUNT(*) FROM user').pluck().get()).toBe(1)
    expect(db.prepare('SELECT COUNT(*) FROM revision').pluck().get()).toBe(1)
  })
})
