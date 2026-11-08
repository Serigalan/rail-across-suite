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
}

/** Write one vector file: [{ input, output }] as JSON. */
export function writeVectors(crate, name, cases) {
  const file = join(CRATES, crate, 'tests/vectors', `${name}.json`)
  mkdirSync(dirname(file), { recursive: true })
  writeFileSync(file, JSON.stringify(cases, null, 1) + '\n')
  return file
}

const wanted = process.argv.slice(2)
for (const [crate, recorders] of Object.entries(RECORDERS)) {
  if (wanted.length && !wanted.includes(crate)) continue
  for (const load of recorders) {
    const vectors = await (await load()).default()
    for (const [name, cases] of Object.entries(vectors)) {
      console.log(`${writeVectors(crate, name, cases)}: ${cases.length} cases`)
    }
  }
}
