#!/usr/bin/env node
/**
 * The import boundaries of the repository (decision 0001):
 *
 * - an app imports packages, never another app;
 * - packages, the server and the services never import an app.
 *
 * An import is found by its specifier: a relative path is resolved to the
 * directory it lands in, a bare one is matched against the `name` in the
 * package.json of each app and package. Comments are skipped; a specifier in
 * a string that is no import can still be reported — rare, and visible.
 *
 * While the Open-Layout-Tool is taken apart, a breach listed in TRANSITIONAL is
 * reported but allowed.
 *
 * Usage: node tools/boundaries.mjs [root]   — exits 1 and lists every breach.
 */
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs'
import { dirname, join, relative, resolve, sep } from 'node:path'
import { fileURLToPath } from 'node:url'

const SOURCE = /\.(?:[cm]?js|jsx|[cm]?ts|tsx)$/
const SKIP = new Set(['node_modules', 'dist', 'dist-local', 'build', 'coverage', '.git'])

/** Every source file below `dir`. */
function sourceFiles(dir) {
  if (!existsSync(dir)) return []
  const out = []
  for (const name of readdirSync(dir)) {
    if (SKIP.has(name)) continue
    const path = join(dir, name)
    if (!existsSync(path)) continue // a link to data that is not fetched
    if (statSync(path).isDirectory()) out.push(...sourceFiles(path))
    else if (SOURCE.test(name)) out.push(path)
  }
  return out
}

/** The area a path lies in: `app:<name>`, `package:<name>`, `server`, `service:<name>`, or null. */
export function areaOf(root, path) {
  const parts = relative(root, path).split(sep)
  if (parts[0] === '..' || parts.length < 2) return null
  if (parts[0] === 'apps') return `app:${parts[1]}`
  if (parts[0] === 'packages') return `package:${parts[1]}`
  if (parts[0] === 'server') return 'server'
  if (parts[0] === 'services') return `service:${parts[1]}`
  return null
}

/** Package names of the apps and packages → their area. */
function packageNames(root) {
  const names = new Map()
  for (const [top, kind] of [['apps', 'app'], ['packages', 'package']]) {
    const dir = join(root, top)
    if (!existsSync(dir)) continue
    for (const name of readdirSync(dir)) {
      const manifest = join(dir, name, 'package.json')
      if (!existsSync(manifest)) continue
      const pkg = JSON.parse(readFileSync(manifest, 'utf8'))
      if (pkg.name) names.set(pkg.name, `${kind}:${name}`)
    }
  }
  return names
}

/** The text with comments blanked out, line breaks kept so line numbers stay right. */
function withoutComments(text) {
  return text
    .replace(/\/\*[\s\S]*?\*\//g, m => m.replace(/[^\n]/g, ' '))
    .replace(/(^|[^:'"`\\])\/\/[^\n]*/g, (m, lead) => lead + ' '.repeat(m.length - lead.length))
}

const SPECIFIER = /(?:\bfrom|\bimport|\brequire)\s*\(?\s*(['"])([^'"\n]+)\1/g

/** The import specifiers of a source text with their line numbers. */
export function importsOf(text) {
  const code = withoutComments(text)
  const out = []
  for (const m of code.matchAll(SPECIFIER)) {
    out.push({ spec: m[2], line: code.slice(0, m.index).split('\n').length })
  }
  return out
}

/** The area an import lands in, or null when it is outside every area (a dependency, node:fs …). */
function targetOf(root, file, spec, names) {
  if (spec.startsWith('.') || spec.startsWith('/')) return areaOf(root, resolve(dirname(file), spec))
  for (const [name, area] of names) {
    if (spec === name || spec.startsWith(`${name}/`)) return area
  }
  return null
}

/** Why an import from area `from` into area `to` is not allowed, or null. */
export function breach(from, to) {
  if (!from || !to || from === to || !to.startsWith('app:')) return null
  return from.startsWith('app:') ? 'an app imports another app' : `${from.split(':')[0]} imports an app`
}

/**
 * Breaches allowed for now, as `from → to`. The project server still runs modules
 * of the Open-Layout-Tool (the project checks, the point clouds) until phase 1 of
 * the roadmap moves them into packages; remove an entry with its last import.
 */
export const TRANSITIONAL = new Set(['server → app:open-layout-tool'])

/** Every breach below `root`: [{ file, line, spec, rule, from, to }]. */
export function checkBoundaries(root) {
  const names = packageNames(root)
  const found = []
  for (const top of ['apps', 'packages', 'server', 'services']) {
    for (const file of sourceFiles(join(root, top))) {
      const from = areaOf(root, file)
      for (const { spec, line } of importsOf(readFileSync(file, 'utf8'))) {
        const to = targetOf(root, file, spec, names)
        const rule = breach(from, to)
        if (rule) found.push({ file: relative(root, file), line, spec, rule, from, to })
      }
    }
  }
  return found
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const root = resolve(process.argv[2] ?? join(dirname(fileURLToPath(import.meta.url)), '..'))
  const all = checkBoundaries(root)
  const allowed = all.filter(f => TRANSITIONAL.has(`${f.from} → ${f.to}`))
  const found = all.filter(f => !allowed.includes(f))
  if (allowed.length) {
    console.log(`boundaries: ${allowed.length} import(s) allowed for now (${[...TRANSITIONAL].join(', ')})`)
  }
  for (const f of found) console.error(`${f.file}:${f.line}: ${f.rule} ('${f.spec}')`)
  if (found.length) {
    console.error(`\n${found.length} import(s) across a boundary. Apps import packages, never each other; ` +
      'packages, the server and the services never import an app (docs/decisions/0001-one-repository.md).')
    process.exit(1)
  }
  console.log('boundaries: ok')
}
