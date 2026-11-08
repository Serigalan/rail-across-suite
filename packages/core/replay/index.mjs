#!/usr/bin/env node
// npm run replay — the JavaScript core's calls recorded as vectors for the Rust
// crates that replace it (decision 0020): each recorder below runs functions
// of the core on the fixtures and real data it is given, and writes what went
// in and what came out to crates/<crate>/tests/vectors/<name>.json, where the
// crate's tests replay them. Run again whenever the JavaScript changes; the
// vectors are in the repository, so the Rust is held to them without Node.
//
//   node replay/index.mjs            all recorders
//   node replay/index.mjs doc        the recorders of one crate
import { mkdirSync, writeFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const here = dirname(fileURLToPath(import.meta.url))
const CRATES = resolve(here, '../../../crates')

/** The recorders, by crate: each a module whose default export returns { name: cases }. */
const RECORDERS = {
  doc: [() => import('./doc.mjs')],
  geom: [() => import('./geom.mjs')],
  rules: [() => import('./rules.mjs')],
  track: [() => import('./track.mjs')],
  survey: [() => import('./survey.mjs')],
  views: [() => import('./views.mjs')],
  switch: [() => import('./switch.mjs')],
  commands: [() => import('./commands.mjs')],
  merge: [() => import('./merge.mjs')],
  signalling: [() => import('./signalling.mjs')],
  planpro: [() => import('./planpro.mjs')],
  pointcloud: [() => import('./pointcloud.mjs')],
}

/** Write one vector file: [{ op, input, output | error }], one case a line. */
export function writeVectors(crate, name, cases) {
  const file = join(CRATES, crate, 'tests/vectors', `${name}.json`)
  mkdirSync(dirname(file), { recursive: true })
  writeFileSync(file, `[\n${cases.map(c => JSON.stringify(c)).join(',\n')}\n]\n`)
  return file
}

const wanted = process.argv.slice(2)
for (const [crate, recorders] of Object.entries(RECORDERS)) {
  if (wanted.length && !wanted.includes(crate)) continue
  for (const load of recorders) {
    const vectors = await (await load()).default()
    // A recorder writing into several crates answers { crate: { name: cases } }.
    const byCrate = Object.values(vectors).every(v => !Array.isArray(v)) ? vectors : { [crate]: vectors }
    for (const [into, files] of Object.entries(byCrate)) {
      for (const [name, cases] of Object.entries(files)) {
        console.log(`${writeVectors(into, name, cases)}: ${cases.length} cases`)
      }
    }
  }
}
