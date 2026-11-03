import { execFile } from 'node:child_process'
import { mkdtemp, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

// An Access database (DB ASCII interface, Satzarten 11–41) turned into the
// import payload. The browser cannot read an Access file, so the conversion
// runs here (decision 11 of the Layout-Tool). Nothing of its meaning is
// decided here: the tables that matter are lifted out under short names;
// what the values mean — element types, Bauformen, Lagesysteme — is decided
// in the Layout-Tool's utils/exchange/mdbImport.js, where it is testable
// without a database. Reading is done with `mdb-export` from mdbtools, one
// child process per table.

// Access files start with this; checked before a child process is started so
// a wrong upload fails here instead of inside mdbtools.
const MAGIC = Buffer.from('\x00\x01\x00\x00Standard Jet DB', 'latin1')
const MAGIC_ACE = Buffer.from('\x00\x01\x00\x00Standard ACE DB', 'latin1')
const TIMEOUT = Number(process.env.RA_MDB_TIMEOUT ?? 180) * 1000
const TOOL = process.env.RA_MDB_EXPORT ?? 'mdb-export'

/** A conversion that failed; `code` is the key the client translates. */
export class MdbError extends Error {
  constructor(code, message) {
    super(message ?? code)
    this.code = code
  }
}

/** Rows of a CSV as mdb-export writes it: a header, fields quoted with ", quotes doubled, line breaks inside quotes. */
export function parseCsv(text) {
  const rows = []
  let row = []
  let field = ''
  let quoted = false
  for (let i = 0; i < text.length; i++) {
    const c = text[i]
    if (quoted) {
      if (c === '"' && text[i + 1] === '"') { field += '"'; i++ } else if (c === '"') quoted = false
      else field += c
    } else if (c === '"') quoted = true
    else if (c === ',') { row.push(field); field = '' } else if (c === '\n' || c === '\r') {
      if (c === '\r' && text[i + 1] === '\n') i++
      row.push(field)
      rows.push(row)
      row = []
      field = ''
    } else field += c
  }
  if (field !== '' || row.length) { row.push(field); rows.push(row) }
  const [head = [], ...body] = rows.filter(r => r.length > 1 || r[0] !== '')
  return body.map(r => Object.fromEntries(head.map((h, k) => [h, r[k] ?? ''])))
}

const num = (v) => {
  if (v == null || String(v).trim() === '') return null
  const x = Number(v)
  return Number.isFinite(x) ? x : null
}
const int = (v, fallback = 0) => {
  const x = num(v)
  return x == null ? fallback : Math.trunc(x)
}
const txt = (v) => (v ?? '').trim()
const mp = (mantissa, exponent) => {
  const m = num(mantissa)
  return m == null ? null : m * 10 ** int(exponent)
}

/** One table as rows, without the records GND-Edit marked as deleted (`bDel`). A table the file does not have yields nothing. */
function rows(path, table) {
  return new Promise((resolve, reject) => {
    execFile(TOOL, [path, table], { timeout: TIMEOUT, maxBuffer: 1 << 30, encoding: 'utf8' }, (err, stdout) => {
      if (err?.code === 'ENOENT') return reject(new MdbError('internal', 'mdb-export not installed'))
      if (err?.killed) return reject(new MdbError('timeout'))
      if (err) return resolve([])
      resolve(parseCsv(stdout).filter(r => int(r.bDel) === 0))
    })
  })
}

/** The payload the Layout-Tool's `parseMdbPayload` reads, from the bytes of an Access file. */
export async function convertMdb(bytes) {
  const head = bytes.subarray(0, MAGIC.length)
  if (!head.equals(MAGIC) && !head.equals(MAGIC_ACE)) throw new MdbError('not_a_database')
  // The file leaves the user's machine to get here, so it lives exactly as
  // long as the conversion does.
  const dir = await mkdtemp(join(tmpdir(), 'ra-mdb-'))
  const path = join(dir, 'upload.mdb')
  try {
    await writeFile(path, bytes, { mode: 0o600 })
    const t = (table) => rows(path, table)

    // The format version (4, or 6 for AGON); the same in every row.
    const version = (await t('TABSYS_GNDED')).map(r => int(r.TABSYS_VERSION, null)).find(v => v != null) ?? null

    // `mp` is the mean point error in metres, stored as mantissa and exponent.
    const points = (await t('X_ASC12_PL'))
      .map(r => ({ pad: txt(r.PAD), sys: txt(r.LSYS), y: num(r.Y), x: num(r.X), mp: mp(r.MP, r.MPEXP) }))
      .filter(p => p.pad && p.y != null && p.x != null)

    const elements = (await t('X_ASC21_EL')).map(r => ({
      pad1: txt(r.PAD1), pad2: txt(r.PAD2), sys: txt(r.ELSYS), typ: int(r.ELTYP, -1),
      p1: num(r.ELPAR1), p2: num(r.ELPAR2), p3: num(r.ELPAR3), ariwi: num(r.ELARIWI),
      // The alignment designation the planning tool writes ("Trasse:…").
      text: txt(r.ELTEXT), err: int(r.ErrStatus),
    }))

    const cants = (await t('X_ASC23_EU')).map(r => ({
      pad1: txt(r.PAD1), pad2: txt(r.PAD2), typ: int(r.EUTYP, -1),
      p1: num(r.EUPAR1), p2: num(r.EUPAR2), p3: num(r.EUPAR3), err: int(r.ErrStatus),
    }))

    // A Gleisabschnitt names its elements through an ordered list of point
    // addresses in a child table, linked by the parser's running number.
    const chains = new Map()
    for (const r of await t('X_ASC33_GS_L')) {
      const key = txt(r.ParseLfdNr)
      if (!chains.has(key)) chains.set(key, [])
      chains.get(key).push([int(r.LfdNr), txt(r.L)])
    }
    const tracks = (await t('X_ASC33_GS')).map(r => {
      const key = txt(r.ParseLfdNr)
      const chain = (chains.get(key) ?? []).sort((a, b) => a[0] - b[0]).map(([, pad]) => pad).filter(Boolean)
      return {
        id: key, a: txt(r.AKNOTEN), e: txt(r.EKNOTEN), strecke: txt(r.STRECKE), gleis: txt(r.GLEISNR),
        richtung: int(r.STRRIKZ, 0), chain, err: int(r.ErrStatus),
      }
    })

    // Satzart 11: what a point address is stationed at, packed as it stands.
    const stations = (await t('X_ASC11_PP'))
      .map(r => ({ pad: txt(r.PAD), station: num(r.STATION), strecke: txt(r.PSTRECKE), richtung: int(r.PSTRRIKZ, 0) }))
      .filter(s => s.pad)

    // Satzart 13: the height of a point, one record per height system.
    const heights = (await t('X_ASC13_PH'))
      .map(r => ({ pad: txt(r.PAD), sys: txt(r.HSYS), h: num(r.H) }))
      .filter(h => h.pad && h.h != null)

    // Satzart 22: the gradient, as a chain between point addresses.
    const gradients = (await t('X_ASC22_EH')).map(r => ({
      pad1: txt(r.PAD1), pad2: txt(r.PAD2), sys: txt(r.EHSYS), typ: int(r.EHTYP, -1),
      p1: num(r.EHPAR1), p2: num(r.EHPAR2), p3: num(r.EHPAR3), err: int(r.ErrStatus),
    }))

    // Satzart 24: the kilometrage line itself, with the packed kilometrage at both ends.
    const km = (await t('X_ASC24_EK')).map(r => ({
      pad1: txt(r.PAD1), pad2: txt(r.PAD2), sys: txt(r.EKSYS), typ: int(r.EKTYP, -1),
      p1: num(r.EKPAR1), p2: num(r.EKPAR2), p3: num(r.EKPAR3), ariwi: num(r.EKARIWI),
      akm: num(r.EKAKM), ekm: num(r.EKEKM),
    }))

    const nodes = (await t('X_ASC31_KN')).map(r => ({
      knoten: r.KNOTEN ?? '', typ: int(r.KNTYP, -1), pad: txt(r.PAD), form: txt(r.KNBE),
      nb: ['KN0', 'KN1', 'KN2', 'KN3'].map(k => txt(r[k])), err: int(r.ErrStatus),
    }))

    if (!elements.length && !nodes.length) throw new MdbError('no_records')
    const all = { points, elements, cants, tracks, nodes, stations, heights, gradients, km }
    return { version, ...all, counts: Object.fromEntries(Object.entries(all).map(([k, v]) => [k, v.length])) }
  } finally {
    await rm(dir, { recursive: true, force: true })
  }
}
