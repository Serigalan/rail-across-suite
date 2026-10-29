#!/usr/bin/env node
// The data kept out of the repository's history (data.json): each file is
// fetched to its path from the first address that answers, and written only
// when its SHA-256 is the one named there. A file already in place with the
// right hash is left alone, so this runs before every test and build at no cost.
// A link of data.json is made where its target is there and removed where it
// is not: the kilometrage lines reach public/data/ only where someone put them.
//
//   node tools/data.mjs           fetch what is missing or different
//   node tools/data.mjs --check   only say what is missing, exit 1 if anything is
import { createHash } from 'node:crypto'
import { existsSync, lstatSync, mkdirSync, readFileSync, renameSync, rmSync, symlinkSync, writeFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..')

const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex')

/** True when the file at `path` is there with the hash `want`. */
export function inPlace(path, want) {
  return existsSync(path) && sha256(readFileSync(path)) === want
}

/** The file's bytes from the first of its addresses that gives the right hash. */
async function fetchOne(file) {
  const tried = []
  for (const url of file.urls) {
    try {
      const res = await fetch(url)
      if (!res.ok) { tried.push(`${url}: HTTP ${res.status}`); continue }
      const bytes = Buffer.from(await res.arrayBuffer())
      const got = sha256(bytes)
      if (got === file.sha256) return bytes
      tried.push(`${url}: SHA-256 ${got}`)
    } catch (err) {
      tried.push(`${url}: ${err.cause?.code ?? err.message}`)
    }
  }
  throw new Error(`${file.path}: no address gave SHA-256 ${file.sha256}\n  ${tried.join('\n  ')}`)
}

/** Make or remove the link at `path` (relative to the app) to `target` (relative to the link). */
export function placeLink(root, { path, target }) {
  const link = join(root, path)
  const there = existsSync(resolve(dirname(link), target))
  let present = false
  try { present = lstatSync(link).isSymbolicLink() } catch { /* not there */ }
  if (present && !there) rmSync(link)
  if (!present && there) symlinkSync(target, link)
  return there
}

async function main() {
  const check = process.argv.includes('--check')
  const { files, links = [] } = JSON.parse(readFileSync(join(ROOT, 'data.json'), 'utf8'))
  let failed = 0
  for (const file of files) {
    const path = join(ROOT, file.path)
    if (inPlace(path, file.sha256)) continue
    if (check) { console.error(`missing or different: ${file.path}`); failed++; continue }
    try {
      const bytes = await fetchOne(file)
      mkdirSync(dirname(path), { recursive: true })
      writeFileSync(`${path}.part`, bytes)
      renameSync(`${path}.part`, path)
      console.log(`fetched ${file.path} (${bytes.length} bytes)`)
    } catch (err) {
      console.error(err.message)
      failed++
    }
  }
  if (failed) process.exit(1)
  const linked = check ? [] : links.filter(l => placeLink(ROOT, l)).map(l => l.path)
  console.log(`data: ${files.length} file(s) in place` + (linked.length ? `, linked ${linked.join(', ')}` : ''))
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) await main()
