#!/usr/bin/env node
// Der Anhang der Formatbeschreibung rail-across-1.0 (rail-across-1.0-anhang.typ), aus den Schemas erzeugt:
// jeder Objekttyp mit jedem Property, ob es Pflicht ist, seinem Typ mit den zulässigen Werten und seiner
// Bedeutung (bedeutungen.mjs). Läuft wieder, wann immer ein Schema sich ändert.
//
//   node docs/format/anhang.mjs           schreiben
//   node docs/format/anhang.mjs --check   Exit 1, wo die Datei nicht mehr ist, was die Schemas ergeben
import { readFileSync, writeFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import MEANINGS from './bedeutungen.mjs'

const HERE = dirname(fileURLToPath(import.meta.url))
const SCHEMAS = resolve(HERE, '../../schemas')
const OUT = join(HERE, 'rail-across-1.0-anhang.typ')

/** Die Schemas in der Reihenfolge des Anhangs: [Name, Datei, Überschrift, Name des Wurzeltyps]. */
const PARTS = [
  ['infra-data', 'infra-data.schema.json', 'Hülle von infra-data', 'Datei'],
  ['core', 'core/3.1.schema.json', 'Domäne core', 'Domänendokument'],
  ['civil', 'civil/2.1.schema.json', 'Layer civil', 'Layerdokument'],
  ['signalling', 'signalling/3.1.schema.json', 'Layer signalling', 'Layerdokument'],
  ['electrification', 'electrification/2.1.schema.json', 'Layer electrification', 'Layerdokument'],
  ['operations-file', 'operations.schema.json', 'Hülle von operations', 'Datei'],
  ['operations', 'operations/1.0.schema.json', 'Domäne operations', 'Domänendokument'],
]
/** Unter welchem Namen ein Schema über `$id` verwiesen wird. */
const BY_ID = { core: 'core', civil: 'civil', signalling: 'signalling', electrification: 'electrification', operations: 'operations' }

const schemas = Object.fromEntries(PARTS.map(([name, file]) => [name, JSON.parse(readFileSync(join(SCHEMAS, file), 'utf8'))]))
const versionOf = (name, file) => {
  const s = schemas[name]
  return s.properties?.version?.const ?? file.match(/(\d+\.\d+)\.schema\.json$/)?.[1] ?? ''
}

// ── Text ────────────────────────────────────────────────────────────────────

/** Text als Typst-Markup: was zwischen Backticks steht, bleibt Code; sonst werden die Zeichen geschützt, die Typst deutet. */
function esc(text) {
  return String(text ?? '').split(/(`[^`]*`)/).map((part, i) => (i % 2
    ? part
    : part.replace(/[\\*_#[\]$@<>~]/g, (c) => `\\${c}`))).join('')
}
const code = (v) => `\`${typeof v === 'string' ? v : JSON.stringify(v)}\``

const missing = new Set()
/** Die Bedeutung unter `key`, sonst die allgemeine des Property-Namens, sonst die englische des Schemas. */
function meaning(key, name, english) {
  if (MEANINGS[key] != null) return esc(MEANINGS[key])
  if (name && MEANINGS[`*.${name}`] != null) return esc(MEANINGS[`*.${name}`])
  if (english || name) missing.add(key)
  return esc(english ?? '')
}

// ── Typen ───────────────────────────────────────────────────────────────────

/** Der Knoten, auf den ein `$ref` zeigt, und wo er steht: [Schema, Typ, Knoten]. */
function resolveRef(ref, schema) {
  const [file, pointer] = ref.split('#')
  const target = file ? BY_ID[file] : schema
  const def = pointer.replace(/^\/\$defs\//, '')
  return [target, def, schemas[target].$defs?.[def]]
}
const linkTo = (schema, def) => `#link(<${schema}-${def}>)[${code(def)}]`
const isObjectWithProps = (n) => n && typeof n === 'object' && n.properties
const enumKey = (key, v) => `${key}=${v}`
const hasValueMeanings = (key, values) => values.some(v => MEANINGS[enumKey(key, v)] != null)

/** Wohin ein Verweis zeigt. */
function target(node) {
  const t = node['x-target']
  if (!t) return ''
  return /^[a-z-]+\.[a-zA-Z]+$/.test(t) ? ` → ${code(t)}` : ' → Kennung eines anderen Höhenpunkts'
}

/** Typ und zulässige Werte eines Knotens, als Markup. `key` ist der Schlüssel seiner Bedeutung (für Aufzählungen). */
function describe(node, schema, key) {
  if (node === true || node == null || (typeof node === 'object' && !Object.keys(node).filter(k => !['description', 'title'].includes(k)).length)) return 'beliebiger Wert'
  if (node.$ref) {
    const [s, def] = resolveRef(node.$ref, schema)
    return `${linkTo(s, def)}${target(node)}`
  }
  if ('const' in node) return `genau ${code(node.const)}`
  if (node.enum) {
    return hasValueMeanings(key, node.enum) ? 'eins der Werte unter Bedeutung' : `eins von ${node.enum.map(code).join(', ')}`
  }
  if (node.allOf) {
    const base = node.allOf.find(n => n.$ref)
    const any = node.allOf.find(n => n.anyOf)?.anyOf
    if (base && any) {
      const [s, def] = resolveRef(base.$ref, schema)
      return `ein ${linkTo(s, def)} mit mindestens einem von ${any.map(a => a.required.map(code).join(' und ')).join(', ')}`
    }
  }
  if (node.oneOf || node.anyOf) {
    const alts = node.oneOf ?? node.anyOf
    return `eins von: ${alts.map(a => (isObjectWithProps(a) ? 'Objekt (Properties unten)' : describe(a, schema, key))).join('; ')}`
  }
  const types = Array.isArray(node.type) ? node.type : node.type ? [node.type] : []
  if (types.length > 1) return types.map(t => describe({ ...node, type: t }, schema, key)).join(' oder ')
  const t = types[0]
  const bounds = []
  if (node.minimum != null) bounds.push(`≥ ${node.minimum}`)
  if (node.maximum != null) bounds.push(`≤ ${node.maximum}`)
  if (t === 'string') {
    const parts = ['Text']
    if (node.minLength) parts.push(`mindestens ${node.minLength} Zeichen`)
    if (node.pattern) parts.push(`Muster ${code(node.pattern)}`)
    return parts.join('; ')
  }
  if (t === 'number') return ['Zahl', ...bounds].join('; ')
  if (t === 'integer') return ['ganze Zahl', ...bounds].join('; ')
  if (t === 'boolean') return 'Wahrheitswert'
  if (t === 'null') return 'null'
  if (t === 'array') {
    const items = node.items
    let what = 'beliebigen Werten'
    if (isObjectWithProps(items)) what = 'Objekten (Properties unten)'
    else if (items?.oneOf && items.oneOf.every(isObjectWithProps)) what = 'eins der Objekte unten'
    else if (items) what = describe(items, schema, key)
    const counts = []
    if (node.minItems != null && node.maxItems != null && node.minItems === node.maxItems) counts.push(`genau ${node.minItems} Einträge`)
    else {
      if (node.minItems != null) counts.push(`mindestens ${node.minItems} ${node.minItems === 1 ? 'Eintrag' : 'Einträge'}`)
      if (node.maxItems != null) counts.push(`höchstens ${node.maxItems} Einträge`)
    }
    return [`Liste von ${what}`, ...counts].join('; ')
  }
  if (t === 'object' || node.properties || node.patternProperties || node.additionalProperties) {
    if (node.properties) return 'Objekt (Properties unten)'
    if (node.patternProperties) {
      return `Objekt; ${Object.entries(node.patternProperties).map(([p, v]) => `Schlüssel nach ${code(p)}: ${describe(v, schema, key)}`).join('; ')}`
    }
    if (node.additionalProperties && typeof node.additionalProperties === 'object') return `Objekt; Schlüssel frei, Werte: ${describe(node.additionalProperties, schema, key)}`
    return 'Objekt'
  }
  return 'beliebiger Wert'
}

/** Die Bedeutung eines Properties mit den Bedeutungen seiner Werte. */
function propertyMeaning(node, schema, def, path, name) {
  const key = `${schema}:${def}.${path}`
  let text = meaning(key, name, node?.description)
  const values = node?.enum ?? (node?.const != null ? [node.const] : null)
  if (values && hasValueMeanings(key, values)) {
    const lines = values.filter(v => MEANINGS[enumKey(key, v)] != null).map(v => ` #linebreak() ${code(v)} ${esc(MEANINGS[enumKey(key, v)])}`)
    text = node.const != null ? `${text} — ${code(node.const)} ${esc(MEANINGS[enumKey(key, node.const)])}` : text + lines.join('')
  }
  // Wo nur einige Werte eine Bedeutung haben, fehlen die anderen; Kürzel ohne jede (PlanPro-Arten) sprechen für sich.
  if (values && node.enum && hasValueMeanings(key, values)) {
    for (const v of values) if (MEANINGS[enumKey(key, v)] == null) missing.add(enumKey(key, v))
  }
  return text
}

/** Die Zeilen eines Objekts, verschachtelte Objekte mit Punkt, Listeneinträge mit []. */
function rows(node, schema, def, prefix = '', out = [], seen = new Set()) {
  const required = new Set(node.required ?? [])
  for (const [name, prop] of Object.entries(node.properties ?? {})) {
    const path = prefix + name
    const key = `${schema}:${def}.${path}`
    if (!seen.has(path)) {
      seen.add(path)
      out.push([code(path), required.has(name) ? 'ja' : '', describe(prop, schema, key), propertyMeaning(prop, schema, def, path, name)])
    }
    if (isObjectWithProps(prop)) rows(prop, schema, def, `${path}.`, out, seen)
    for (const alt of prop.oneOf ?? []) if (isObjectWithProps(alt)) rows(alt, schema, def, `${path}.`, out, seen)
    const items = prop.items
    if (isObjectWithProps(items)) rows(items, schema, def, `${path}[].`, out, seen)
    for (const alt of items?.oneOf ?? []) if (isObjectWithProps(alt)) rows(alt, schema, def, `${path}[].`, out, seen)
  }
  return out
}

const table = (lines) => [
  '#table(columns: (26%, 8%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },',
  '  [Property], [Pflicht], [Typ und zulässige Werte], [Bedeutung],',
  ...lines.map(cells => `  ${cells.map(c => `[${c}]`).join(', ')},`),
  ')',
].join('\n')

/** Der Abschnitt eines Typs. */
function section(schema, def, node, title) {
  const out = [`=== ${title} <${schema}-${def}>`, '']
  const what = MEANINGS[`TYPE:${schema}:${def}`] != null ? esc(MEANINGS[`TYPE:${schema}:${def}`]) : (missing.add(`TYPE:${schema}:${def}`), esc(node.description ?? ''))
  if (what) out.push(what, '')
  if (isObjectWithProps(node)) {
    if (node.additionalProperties === false) out.push('_Weitere Properties sind nicht erlaubt._', '')
    out.push(table(rows(node, schema, def)), '')
  } else if (node.oneOf && node.oneOf.every(isObjectWithProps)) {
    node.oneOf.forEach((alt, i) => {
      const fixed = Object.entries(alt.properties).find(([, p]) => 'const' in p)
      out.push(`*Variante ${i + 1}*${fixed ? ` (${code(fixed[0])} = ${code(fixed[1].const)})` : ''}:`, '')
      out.push(table(rows(alt, schema, def)), '')
    })
  } else {
    const key = `${schema}:${def}`
    let value = describe(node, schema, key)
    if (node.enum) {
      const m = node.enum.filter(v => MEANINGS[enumKey(key, v)] != null)
      value = `eins von ${node.enum.map(code).join(', ')}${m.length ? `: ${m.map(v => `#linebreak() ${code(v)} ${esc(MEANINGS[enumKey(key, v)])}`).join(' ')}` : ''}`
      if (m.length) for (const v of node.enum) if (MEANINGS[enumKey(key, v)] == null) missing.add(enumKey(key, v))
    }
    out.push(`Wert: ${value}`, '')
  }
  return out.join('\n')
}

// ── Der Anhang ──────────────────────────────────────────────────────────────

function appendix() {
  const out = [
    '// Aus den Schemas erzeugt: node docs/format/anhang.mjs — nicht von Hand ändern.',
    '',
    '= Anhang: alle Properties und ihre Werte <anhang>',
    '',
    `Der Anhang ist aus den Schemas erzeugt (${PARTS.map(([, file]) => code(file)).join(', ')}). Für jeden Objekttyp nennt er jedes Property mit seinem Pfad (verschachtelte Objekte mit Punkt, Listeneinträge mit ${code('[]')}), ob es Pflicht ist, seinen Typ mit allen zulässigen Werten und seine Bedeutung. Ein Verweis auf einen Typ führt zu dessen Abschnitt; ${code('→ core.tracks')} nennt die Sammlung, in die eine Kennung zeigt — die Prüfung beim Einlesen stellt sicher, dass es das Objekt gibt.`,
    '',
  ]
  for (const [name, file, heading, rootTitle] of PARTS) {
    const s = schemas[name]
    out.push(`== ${heading} ${versionOf(name, file)}`, '')
    out.push(section(name, 'root', s, rootTitle))
    for (const [def, node] of Object.entries(s.$defs ?? {})) out.push(section(name, def, node, def))
  }
  // Ein Semikolon direkt hinter einem Link beendete dessen Code-Ausdruck und verschwände: geschützt.
  const text = out.join('\n').replace(/(#link\(<[^>]+>\)\[`[^`]*`\]);/g, '$1\\;')
  return `${text.replace(/\n{3,}/g, '\n\n').trimEnd()}\n`
}

const text = appendix()
if (process.argv[2] === '--check') {
  let now = null
  try { now = readFileSync(OUT, 'utf8') } catch { /* nicht da */ }
  if (now !== text) {
    console.log('docs/format/rail-across-1.0-anhang.typ ist nicht, was die Schemas ergeben — node docs/format/anhang.mjs')
    process.exitCode = 1
  }
} else {
  writeFileSync(OUT, text)
  console.log('docs/format/rail-across-1.0-anhang.typ')
}
if (missing.size) console.error(`ohne deutsche Bedeutung (${missing.size}):\n  ${[...missing].join('\n  ')}`)
