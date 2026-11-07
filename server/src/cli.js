import { dirname, join, resolve } from 'node:path'
import { buildApp } from './app.js'
import { openDatabase } from './db.js'
import { createAuth, passwordAcceptable, randomPassword } from './auth.js'

/**
 * rail-across-server — the project server of Rail-Across-Suite: one for the suite
 * and every module in it (phase 10 of the Layout-Tool, where it began).
 *
 *   rail-across-server serve [--local] [--static <dir>] [--host <h>] [--port <n>]
 *                                                   run the API (RA_SERVER_HOST, RA_SERVER_PORT).
 *                                                   --local: the local package (desktop/) — no user
 *                                                   management, and with --static the built suite
 *                                                   is served beside the API; the bound address is
 *                                                   printed as RA_DESKTOP_URL=<address> (port 0 picks
 *                                                   a free one). RA_SERVER_LOCAL=1 is the same.
 *   rail-across-server create-admin <login> [name]  create an admin; the start password comes
 *                                                   from RA_ADMIN_PASSWORD or is generated and printed
 *   rail-across-server backup <file>                write a consistent copy of the database
 *   rail-across-server user-count                   print how many users exist (setup asks this
 *                                                   before creating the first admin)
 *
 * The database is RA_SERVER_DB (default ./rail-across.sqlite). Started through
 * bin/rail-across-server.mjs.
 */
const env = process.env
const DB = env.RA_SERVER_DB ?? 'rail-across.sqlite'
/** Where point clouds are kept (phase 13): beside the database unless said otherwise. */
const CLOUDS = env.RA_SERVER_CLOUDS ?? join(dirname(resolve(DB)), 'clouds')

/** Options of `serve` out of its arguments: --local, --static <dir>, --host <h>, --port <n>. */
function serveFlags(args) {
  const out = {}
  for (let i = 0; i < args.length; i++) {
    const a = args[i]
    if (a === '--local') out.local = true
    else if (a === '--static') out.static = args[++i]
    else if (a === '--host') out.host = args[++i]
    else if (a === '--port') out.port = args[++i]
    else throw new Error(`unknown option for serve: ${a}`)
  }
  return out
}

async function main([command, ...args]) {
  if (command === 'serve') {
    const flags = serveFlags(args)
    const local = Boolean(flags.local) || env.RA_SERVER_LOCAL === '1'
    const db = openDatabase(DB)
    const app = buildApp({
      db,
      local,
      staticRoot: flags.static ?? (env.RA_SERVER_STATIC || null),
      secureCookie: env.RA_SERVER_INSECURE_COOKIE !== '1',
      ...(env.RA_SERVER_TRUST_PROXY ? { trustProxy: env.RA_SERVER_TRUST_PROXY.split(',').map(s => s.trim()).filter(Boolean) } : {}),
      logger: { level: env.RA_SERVER_LOG ?? 'info' },
      cloudRoot: CLOUDS,
    })
    if (!local) setInterval(() => app.auth.sweep(), 3600 * 1000).unref()
    const close = async () => { await app.close(); db.close(); process.exit(0) }
    process.on('SIGTERM', close)
    process.on('SIGINT', close)
    const address = await app.listen({ host: flags.host ?? env.RA_SERVER_HOST ?? '127.0.0.1', port: Number(flags.port ?? env.RA_SERVER_PORT ?? 8787) })
    // The local package's shell (desktop/) reads this line to open its window.
    if (local) console.log(`RA_DESKTOP_URL=${address}`)
    return
  }
  if (env.RA_SERVER_LOCAL === '1' && (command === 'create-admin' || command === 'user-count')) {
    throw new Error('the local package has no user management (it runs serve --local)')
  }
  if (command === 'create-admin') {
    const [login, ...nameParts] = args
    if (!login) throw new Error('usage: rail-across-server create-admin <login> [name]')
    const db = openDatabase(DB)
    const auth = createAuth(db)
    if (auth.userByLogin(login)) throw new Error(`a user "${login}" exists already`)
    const given = env.RA_ADMIN_PASSWORD
    if (given && !passwordAcceptable(given)) throw new Error('RA_ADMIN_PASSWORD is shorter than 12 characters')
    const password = given ?? randomPassword()
    await auth.createUser({ login, name: nameParts.join(' ') || login, password, role: 'admin', mustChangePassword: true })
    console.log(`admin "${login}" created.`)
    if (!given) console.log(`start password: ${password}  (to be changed at the first sign-in)`)
    db.close()
    return
  }
  if (command === 'backup') {
    const [file] = args
    if (!file) throw new Error('usage: rail-across-server backup <file>')
    const db = openDatabase(DB)
    await db.backup(file)
    db.close()
    console.log(`backup written to ${file}`)
    return
  }
  if (command === 'user-count') {
    const db = openDatabase(DB)
    console.log(db.prepare('select count(*) as n from user').get().n)
    db.close()
    return
  }
  throw new Error('usage: rail-across-server serve [--local] [--static <dir>] [--host <h>] [--port <n>] | create-admin <login> [name] | backup <file> | user-count')
}

main(process.argv.slice(2)).catch(err => {
  console.error(err.message)
  process.exit(1)
})
