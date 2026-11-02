#!/usr/bin/env node
// The types of the core 1.0 document, derived from its schemas: types/document.d.ts,
// for editors and for JSDoc (`@type {import('@rail-across/core/types/document.d.ts').Core.Track}`).
// The schemas are the source; the file is written from them and never by hand.
//
//   node tools/types.mjs           write it
//   node tools/types.mjs --check   exit 1 where it is not what the schemas give now
import { readFileSync, writeFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { schemaOf } from '../src/document/index.js'

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const OUT = join(ROOT, 'types', 'document.d.ts')

/** The namespace a schema's types go into, by its $id. */
const NAMESPACES = { 'rail-across': 'Document', core: 'Core', layout: 'Layout', signalling: 'Signalling' }

const comment = (text, pad) => (text ? `${pad}/** ${text.replace(/\*\//g, '*\\/')} */\n` : '')
const literal = (v) => JSON.stringify(v)

/** The TypeScript type of a schema node, in the namespace of `home`. */
function typeOf(node, home, pad = '') {
  if (node === true || node == null || Object.keys(node).filter(k => !['description', 'x-target'].includes(k)).length === 0) return 'unknown'
  if (node.$ref) {
    const [id, pointer] = node.$ref.split('#')
    const name = pointer.split('/').pop()
    return id && NAMESPACES[id] !== NAMESPACES[home] ? `${NAMESPACES[id]}.${name}` : name
  }
  if (node.const !== undefined) return literal(node.const)
  if (node.enum) return node.enum.map(literal).join(' | ')
  if (node.oneOf) return node.oneOf.map(n => typeOf(n, home, pad)).join(' | ')
  const types = [node.type].flat().filter(Boolean)
  if (!types.length && node.properties) types.push('object')
  return types.map(t => {
    if (t === 'integer' || t === 'number') return 'number'
    if (t === 'array') return `Array<${typeOf(node.items, home, pad)}>`
    if (t === 'object') return objectOf(node, home, pad)
    return t
  }).join(' | ')
}

function objectOf(node, home, pad) {
  const inner = `${pad}  `
  const required = new Set(node.required ?? [])
  const lines = []
  for (const [name, sub] of Object.entries(node.properties ?? {})) {
    lines.push(`${comment(sub.description, inner)}${inner}${/^[A-Za-z_$][\w$]*$/.test(name) ? name : literal(name)}${required.has(name) ? '' : '?'}: ${typeOf(sub, home, inner)}`)
  }
  const patterns = Object.entries(node.patternProperties ?? {})
  if (patterns.length) {
    lines.push(`${inner}/** Names matching ${patterns.map(([p]) => p).join(', ')}. */`)
    lines.push(`${inner}[name: string]: ${[...new Set(patterns.map(([, s]) => typeOf(s, home, inner)))].join(' | ')}`)
  } else if (node.additionalProperties && node.additionalProperties !== false) {
    lines.push(`${inner}[name: string]: ${typeOf(node.additionalProperties, home, inner)}`)
  } else if (node.additionalProperties === undefined && !node.properties) {
    lines.push(`${inner}[name: string]: unknown`)
  }
  return lines.length ? `{\n${lines.join('\n')}\n${pad}}` : 'Record<string, never>'
}

function namespaceOf(schema) {
  const home = schema.$id
  const out = [`${comment(schema.title, '')}export namespace ${NAMESPACES[home]} {`]
  const { $defs = {}, ...root } = schema
  if (root.properties) out.push(`${comment(root.description, '  ')}  export type Root = ${typeOf(root, home, '  ')}`)
  for (const [name, def] of Object.entries($defs)) {
    out.push(`${comment(def.description, '  ')}  export type ${name} = ${typeOf(def, home, '  ')}`)
  }
  out.push('}')
  return out.join('\n')
}

function render() {
  return [
    '// The types of the core 1.0 document, derived from the schemas in schemas/ by',
    '// packages/core/tools/types.mjs. Do not edit: change the schema and run the tool.',
    '',
    ...['rail-across', 'core', 'layout', 'signalling'].map(id => `${namespaceOf(schemaOf(id))}\n`),
  ].join('\n')
}

const text = render()
if (process.argv[2] === '--check') {
  let now = null
  try { now = readFileSync(OUT, 'utf8') } catch { /* not there */ }
  if (now !== text) {
    console.log('types/document.d.ts is not what the schemas give — node tools/types.mjs')
    process.exitCode = 1
  }
} else {
  writeFileSync(OUT, text)
  console.log('types/document.d.ts')
}
