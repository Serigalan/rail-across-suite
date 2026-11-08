import { randomBytes } from 'node:crypto'
import { closeLegacyGaps } from '@rail-across/core/project'
import { checkRecord, normalizedRecord } from './checks.js'

/** The login of the user the server's own revisions name: no one signs in as it. */
export const SYSTEM_LOGIN = 'system'

/**
 * The server's own author, made when it first writes a revision — inactive,
 * with a password nobody knows, so a revision it writes names it and nothing
 * can sign in as it. Not made before: setup asks whether users exist before it
 * creates the first admin (user-count).
 */
function systemUser(db, now) {
  const get = db.prepare('SELECT id FROM user WHERE login = ?').pluck()
  const id = get.get(SYSTEM_LOGIN)
  if (id != null) return id
  db.prepare(`INSERT INTO user (login, name, password_hash, role, active, must_change_password, created_at)
              VALUES (?, 'Rail-Across-Suite', ?, 'user', 0, 0, ?)`)
    .run(SYSTEM_LOGIN, randomBytes(32).toString('hex'), new Date(now()).toISOString())
  return get.get(SYSTEM_LOGIN)
}

/**
 * Decision 0020: the same place is 1 µm, no longer 1 mm. Every head of every
 * variant still worked on is closed where its joints missed by up to a
 * millimetre (closeLegacyGaps), as a revision of its own the history shows — so
 * what was joined stays joined, and nobody's next check-in carries the change
 * unannounced. A head that is closed already, or in a format no longer read,
 * is left as it is.
 */
function closeNodeGaps(db, store, now) {
  const projects = db.prepare('SELECT id FROM project WHERE deleted_at IS NULL ORDER BY created_at').pluck().all()
  let written = 0, joints = 0
  for (const projectId of projects) {
    for (const variant of store.variants(projectId)) {
      if (variant.archived) continue
      let head
      try { head = store.revision(variant.head_revision_id) } catch { continue }
      const { record, closed } = closeLegacyGaps(normalizedRecord(head.payload))
      if (!closed) continue
      const checked = checkRecord(record)
      const result = store.checkIn({
        variant, base: variant.head_revision_id, authorId: systemUser(db, now),
        message: `${closed} Stöße bis 1 mm geschlossen – derselbe Ort ist jetzt 1 µm (Entscheidung 0020).`,
        payload: checked.record, errorKeys: checked.errors.map(e => e.key), remaps: [],
      })
      if (result.revisionId) { written += 1; joints += closed }
    }
  }
  return `${written} Revisionen, ${joints} Stöße`
}

/** The steps, in the order they run; a name once recorded is never run again. */
const STEPS = [
  ['0020-node-tolerance-1um', closeNodeGaps],
]

/**
 * Run the data steps this database has not seen, each once, and record what
 * each did. The server runs them when it starts, before it listens.
 */
export function runDataMigrations(db, store, { now = () => Date.now() } = {}) {
  const done = new Set(db.prepare('SELECT name FROM data_migration').pluck().all())
  const ran = []
  for (const [name, step] of STEPS) {
    if (done.has(name)) continue
    const note = step(db, store, now)
    db.prepare('INSERT INTO data_migration (name, applied_at, note) VALUES (?, ?, ?)')
      .run(name, new Date(now()).toISOString(), note)
    ran.push({ name, note })
  }
  return ran
}
