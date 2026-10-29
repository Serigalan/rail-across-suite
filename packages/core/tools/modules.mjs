#!/usr/bin/env node
// The modules of the core (packages/core/src) and the rules between them —
// those the Open-Layout-Tool held its computing modules to, now for the core:
//
//   1. Code outside a module imports from its index.js and nowhere else.
//      Tests may reach inside (they test the parts), and a module's own files
//      import each other directly.
//   2. The modules depend on each other without a cycle.
//   3. The core imports nothing outside the package but its dependencies
//      (package.json) and Node's own modules; tests may use vitest.
//   4. Every module has its page in docs/.
//
//   node tools/modules.mjs           check, exit 1 on a breach
//   node tools/modules.mjs --order   the modules in dependency order, with what each needs
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs'
import { dirname, join, relative, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const SRC = join(ROOT, 'src')

const files = (dir) => readdirSync(dir).flatMap(name => {
  const path = join(dir, name)
  return statSync(path).isDirectory() ? files(path) : [path]
})
const isTest = (p) => /\.test\.js$/.test(p) || p.startsWith(join(ROOT, 'test') + '/')

export const MODULES = readdirSync(SRC).filter(n => statSync(join(SRC, n)).isDirectory()).sort()
const moduleOf = (path) => (path.startsWith(SRC + '/') ? path.slice(SRC.length + 1).split('/')[0] : null)

// Static and dynamic imports, re-exports and the files a module opens by URL.
const SPECIFIERS = [
  /\bfrom\s+'([^']+)'/g,
  /\bimport\s+'([^']+)'/g,
  /\bimport\(\s*'([^']+)'\s*\)/g,
  /new URL\(\s*'([^']+)'\s*,\s*import\.meta\.url\s*\)/g,
  /\bvi\.mock\(\s*'([^']+)'/g,
]
const specifiersOf = (source) => SPECIFIERS.flatMap(re => [...source.matchAll(re)].map(m => m[1]))

/** The file or directory a relative specifier names. */
function resolveRelative(from, specifier) {
  const base = resolve(dirname(from), specifier.replace(/\?.*$/, ''))
  for (const suffix of ['', '.js', '.json', '/index.js']) {
    if (existsSync(base + suffix) && statSync(base + suffix).isFile()) return base + suffix
  }
  return base
}

const manifest = JSON.parse(readFileSync(join(ROOT, 'package.json'), 'utf8'))
const DEPENDENCIES = new Set(Object.keys(manifest.dependencies ?? {}))
const packageOf = (spec) => spec.split('/').slice(0, spec.startsWith('@') ? 2 : 1).join('/')

/** The breaches of rules 1–4, as lines of text, and the module graph. */
export function checkModules() {
  const breaches = []
  const needs = new Map(MODULES.map(m => [m, new Set()]))
  const sources = [...files(SRC), ...(existsSync(join(ROOT, 'test')) ? files(join(ROOT, 'test')) : [])]
    .filter(p => /\.(js|mjs)$/.test(p))
  for (const path of sources) {
    const from = moduleOf(path)
    const where = relative(ROOT, path)
    for (const spec of specifiersOf(readFileSync(path, 'utf8'))) {
      if (!spec.startsWith('.')) {
        // Rule 3: dependencies by name.
        if (spec.startsWith('node:') || DEPENDENCIES.has(packageOf(spec))) continue
        if (isTest(path) && packageOf(spec) === 'vitest') continue
        breaches.push(`${where} imports '${spec}' — not a dependency of the core`)
        continue
      }
      const dep = resolveRelative(path, spec)
      // Rule 3: nothing outside the package.
      if (relative(ROOT, dep).startsWith('..')) {
        breaches.push(`${where} imports ${relative(ROOT, dep)} — outside the core`)
        continue
      }
      const to = moduleOf(dep)
      if (!to || to === from) continue
      if (from && !isTest(path)) needs.get(from).add(to)
      // Rule 1: from outside, only the index.
      if (!isTest(path) && dep !== join(SRC, to, 'index.js')) {
        breaches.push(`${where} imports ${relative(ROOT, dep)} — import from src/${to} (its index.js)`)
      }
    }
  }
  // Rule 2: no cycle. Kahn's order; what is left over lies on a cycle.
  const order = []
  const left = new Map([...needs].map(([m, s]) => [m, new Set(s)]))
  while (left.size) {
    const ready = [...left].filter(([, s]) => [...s].every(d => !left.has(d))).map(([m]) => m).sort()
    if (!ready.length) {
      breaches.push(`the modules ${[...left.keys()].join(', ')} need each other in a cycle`)
      break
    }
    for (const m of ready) { order.push(m); left.delete(m) }
  }
  // Rule 4, and the index itself.
  for (const m of MODULES) {
    if (!existsSync(join(SRC, m, 'index.js'))) breaches.push(`src/${m} has no index.js`)
    if (!existsSync(join(ROOT, 'docs', `${m}.md`))) breaches.push(`src/${m} has no page docs/${m}.md`)
  }
  return { breaches, order, needs }
}

function main() {
  const { breaches, order, needs } = checkModules()
  if (process.argv[2] === '--order') {
    for (const m of order) console.log(`${m.padEnd(13)} ${[...needs.get(m)].sort().join(' ')}`)
    return
  }
  for (const b of breaches) console.log(b)
  process.exitCode = breaches.length ? 1 : 0
  if (!breaches.length) console.log(`modules: ${MODULES.length} modules, no breach`)
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main()
