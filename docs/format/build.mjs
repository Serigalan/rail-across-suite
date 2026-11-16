#!/usr/bin/env node
// The format description rail-across-1.0 (docs/format), in English and in its German translation, each as
// Typst and as Markdown: the main text from text/<lang>.md, the appendix — every type with every property,
// whether it is required, its type with the values it allows and its meaning — generated from the
// specification's schemas (schemas/), the meanings from meanings.<lang>.mjs. Run again whenever a text, a
// schema or a meaning changes.
//
//   node docs/format/build.mjs           write rail-across-1.0.{typ,md} and rail-across-1.0.de.{typ,md}
//   node docs/format/build.mjs --check   exit 1 where a file is not what the sources make
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import EN from './meanings.en.mjs'
import DE from './meanings.de.mjs'
import { mainMd, mainTyp, preambleTyp, writeOrCheck } from '../markup.mjs'

const HERE = dirname(fileURLToPath(import.meta.url))
/** What the files are generated from, as their first line says. */
const SOURCE = (lang) => `docs/format/build.mjs from text/${lang}.md and the specification's schemas`
const DATE = { en: '13 November 2026', de: '13. November 2026', short: { en: '2026-11-13', de: '13.11.2026' } }

/** The schemas in the order of the appendix: [name, file, heading key, root title key]. */
const PARTS = [
  ['rail-across', 'rail-across.schema.json', 'file', 'theFile'],
  ['core', 'core/5.0.schema.json', 'domain', 'domainDocument'],
  ['civil', 'civil/3.0.schema.json', 'layer', 'layerDocument'],
  ['signalling', 'signalling/4.0.schema.json', 'layer', 'layerDocument'],
  ['electrification', 'electrification/3.0.schema.json', 'layer', 'layerDocument'],
  ['vehicles', 'vehicles/1.0.schema.json', 'domain', 'domainDocument'],
  ['timetables', 'timetables/1.2.schema.json', 'domain', 'domainDocument'],
]
const schemas = Object.fromEntries(PARTS.map(([name, file]) => [name, JSON.parse(readFileSync(join(HERE, 'schemas', file), 'utf8'))]))
const versionOf = (file) => file.match(/(\d+\.\d+)\.schema\.json$/)?.[1] ?? '1.0'

const WORDS = {
  en: {
    title: 'rail-across-1.0', subtitle: 'An exchange format for railway infrastructure and its operation — format description',
    docTitle: 'rail-across-1.0 — Format description', status: `As of ${DATE.en} · appendix generated from the schemas`,
    header: `As of ${DATE.short.en}`, lang: 'en',
    cols: ['Property', 'Required', 'Type and allowed values', 'Meaning'], yes: 'yes',
    text: 'text', number: 'number', integer: 'integer', boolean: 'boolean', any: 'any value', object: 'object',
    objectBelow: 'object (properties below)', objectsBelow: 'objects (properties below)', oneObjectBelow: 'one of the objects below',
    listOf: 'list of', anyValues: 'any values', exactly: 'exactly', oneOf: 'one of', oneOfColon: 'one of:',
    valuesBelow: 'one of the values under Meaning', minChars: (n) => `at least ${n} characters`, pattern: 'pattern',
    minItems: (n) => `at least ${n} ${n === 1 ? 'entry' : 'entries'}`, maxItems: (n) => `at most ${n} entries`,
    exactItems: (n) => `exactly ${n} entries`, keysAfter: 'keys after', keysFree: 'keys free, values', or: 'or',
    withOne: (base, alts) => `a ${base} with at least one of ${alts}`, requiredOne: (alts) => `exactly one of ${alts} is stated`,
    variant: 'Variant', closed: '_No further properties are allowed._', value: 'Value:',
    file: 'The file rail-across', domain: 'Domain', layer: 'Layer', theFile: 'The file', domainDocument: 'Domain document',
    layerDocument: 'Layer document',
    appendix: 'Appendix: every property and its values',
    intro: (files) => `The appendix is generated from the specification's schemas (${files}, in \`docs/format/schemas\`). For every type it names every property with its path (nested objects with a dot, list entries with \`[]\`), whether it is required, its type with every value it allows, and its meaning. A reference to a type leads to its section; \`→ core.tracks\` names the collection an id points into — the check on reading makes sure the object is there.`,
    missing: 'without a meaning',
  },
  de: {
    title: 'rail-across-1.0', subtitle: 'Ein Austauschformat für Eisenbahninfrastruktur und ihren Betrieb — Formatbeschreibung',
    docTitle: 'rail-across-1.0 — Formatbeschreibung', status: `Stand ${DATE.de} · Anhang aus den Schemas erzeugt · Übersetzung des englischen Originals`,
    header: `Stand ${DATE.short.de}`, lang: 'de',
    cols: ['Property', 'Pflicht', 'Typ und zulässige Werte', 'Bedeutung'], yes: 'ja',
    text: 'Text', number: 'Zahl', integer: 'ganze Zahl', boolean: 'Wahrheitswert', any: 'beliebiger Wert', object: 'Objekt',
    objectBelow: 'Objekt (Properties unten)', objectsBelow: 'Objekten (Properties unten)', oneObjectBelow: 'eins der Objekte unten',
    listOf: 'Liste von', anyValues: 'beliebigen Werten', exactly: 'genau', oneOf: 'eins von', oneOfColon: 'eins von:',
    valuesBelow: 'eins der Werte unter Bedeutung', minChars: (n) => `mindestens ${n} Zeichen`, pattern: 'Muster',
    minItems: (n) => `mindestens ${n} ${n === 1 ? 'Eintrag' : 'Einträge'}`, maxItems: (n) => `höchstens ${n} Einträge`,
    exactItems: (n) => `genau ${n} Einträge`, keysAfter: 'Schlüssel nach', keysFree: 'Schlüssel frei, Werte', or: 'oder',
    withOne: (base, alts) => `ein ${base} mit mindestens einem von ${alts}`, requiredOne: (alts) => `genau eins von ${alts} ist angegeben`,
    variant: 'Variante', closed: '_Weitere Properties sind nicht erlaubt._', value: 'Wert:',
    file: 'Die Datei rail-across', domain: 'Domäne', layer: 'Layer', theFile: 'Die Datei', domainDocument: 'Domänendokument',
    layerDocument: 'Layerdokument',
    appendix: 'Anhang: alle Properties und ihre Werte',
    intro: (files) => `Der Anhang ist aus den Schemas der Spezifikation erzeugt (${files}, in \`docs/format/schemas\`). Für jeden Objekttyp nennt er jedes Property mit seinem Pfad (verschachtelte Objekte mit Punkt, Listeneinträge mit \`[]\`), ob es Pflicht ist, seinen Typ mit allen zulässigen Werten und seine Bedeutung. Ein Verweis auf einen Typ führt zu dessen Abschnitt; \`→ core.tracks\` nennt die Sammlung, in die eine Kennung zeigt — die Prüfung beim Einlesen stellt sicher, dass es das Objekt gibt.`,
    missing: 'ohne Bedeutung',
  },
}

// ── The appendix as a neutral structure ─────────────────────────────────────
// Cell text is plain text with `code` spans; LINK(schema-Def|Def) a link to a type; BR a line break;
// SOFT inside a code span a place where a long name may break.
const LINK = (schema, def) => `\u27e6${schema}-${def}|${def}\u27e7`
const BR = '\u23ce'
const SOFT = '\u200b'
const code = (v) => `\`${typeof v === 'string' ? v : JSON.stringify(v)}\``
// A property path may break after each dot.
const pathCode = (path) => code(path.replaceAll('.', `.${SOFT}`))

function resolveRef(ref, schema) {
  const [file, pointer] = ref.split('#')
  const target = file || schema
  const def = pointer.replace(/^\/\$defs\//, '')
  return [target, def, schemas[target]?.$defs?.[def]]
}
const isObj = (n) => n && typeof n === 'object' && n.properties

function appendixOf(lang) {
  const W = WORDS[lang]
  const M = lang === 'en' ? EN : DE
  const missing = new Set()
  const meaning = (key, name) => {
    if (M[key] != null) return M[key]
    if (name && M[`*.${name}`] != null) return M[`*.${name}`]
    missing.add(key)
    return ''
  }
  const valueKey = (key, v) => `${key}=${v}`
  const hasValues = (key, values) => values.some(v => M[valueKey(key, v)] != null)
  const target = (node) => {
    if (!node['x-target']) return ''
    return ` → ${pathCode(node['x-target'])}`
  }

  function describe(node, schema, key) {
    if (node === true || node == null || (typeof node === 'object' && !Object.keys(node).some(k => !['description', 'title'].includes(k)))) return W.any
    if (node.$ref) {
      const [s, def] = resolveRef(node.$ref, schema)
      return `${LINK(s, def)}${target(node)}`
    }
    if ('const' in node) return `${W.exactly} ${code(node.const)}`
    if (node.enum) return hasValues(key, node.enum) ? W.valuesBelow : `${W.oneOf} ${node.enum.map(code).join(', ')}`
    if (node.allOf) {
      const base = node.allOf.find(n => n.$ref)
      const any = node.allOf.find(n => n.anyOf)?.anyOf
      if (base && any) {
        const [s, def] = resolveRef(base.$ref, schema)
        return W.withOne(LINK(s, def), any.map(a => a.required.map(code).join(' + ')).join(', '))
      }
    }
    if (node.oneOf && !node.type && !node.properties) {
      return `${W.oneOfColon} ${node.oneOf.map(a => (isObj(a) ? W.objectBelow : describe(a, schema, key))).join('; ')}`
    }
    const types = Array.isArray(node.type) ? node.type : node.type ? [node.type] : []
    if (types.length > 1) return types.map(t => describe({ ...node, type: t }, schema, key)).join(` ${W.or} `)
    const t = types[0]
    const bounds = [node.minimum != null && `≥ ${node.minimum}`, node.maximum != null && `≤ ${node.maximum}`].filter(Boolean)
    if (t === 'string') return [W.text, node.minLength && W.minChars(node.minLength), node.pattern && `${W.pattern} ${code(node.pattern)}`].filter(Boolean).join('; ')
    if (t === 'number') return [W.number, ...bounds].join('; ')
    if (t === 'integer') return [W.integer, ...bounds].join('; ')
    if (t === 'boolean') return W.boolean
    if (t === 'null') return 'null'
    if (t === 'array') {
      const items = node.items
      let what = W.anyValues
      if (isObj(items)) what = W.objectsBelow
      else if (items?.oneOf?.every(isObj)) what = W.oneObjectBelow
      else if (items) what = describe(items, schema, key)
      const counts = node.minItems != null && node.minItems === node.maxItems
        ? [W.exactItems(node.minItems)]
        : [node.minItems != null && W.minItems(node.minItems), node.maxItems != null && W.maxItems(node.maxItems)].filter(Boolean)
      return [`${W.listOf} ${what}`, ...counts].join('; ')
    }
    if (t === 'object' || node.properties || node.patternProperties || node.additionalProperties) {
      if (node.properties && Object.keys(node.properties).length) return W.objectBelow
      if (node.patternProperties) return `${W.object}; ${Object.entries(node.patternProperties).map(([p, v]) => `${W.keysAfter} ${code(p)}: ${describe(v, schema, key)}`).join('; ')}`
      if (node.additionalProperties && typeof node.additionalProperties === 'object') return `${W.object}; ${W.keysFree}: ${describe(node.additionalProperties, schema, key)}`
      return W.object
    }
    return W.any
  }

  function propertyMeaning(node, schema, def, path, name) {
    const key = `${schema}:${def}.${path}`
    let text = meaning(key, name)
    const values = node?.enum ?? (node?.const != null ? [node.const] : null)
    if (values && hasValues(key, values)) {
      text = node.const != null
        ? `${text} — ${code(node.const)} ${M[valueKey(key, node.const)]}`
        : text + values.filter(v => M[valueKey(key, v)] != null).map(v => `${BR}${code(v)} ${M[valueKey(key, v)]}`).join('')
      for (const v of values) if (M[valueKey(key, v)] == null) missing.add(valueKey(key, v))
    }
    return text
  }

  function rows(node, schema, def, prefix = '', out = [], seen = new Set()) {
    const required = new Set(node.required ?? [])
    for (const [name, prop] of Object.entries(node.properties ?? {})) {
      const path = prefix + name
      if (!seen.has(path)) {
        seen.add(path)
        out.push([pathCode(path), required.has(name) ? W.yes : '', describe(prop, schema, `${schema}:${def}.${path}`), propertyMeaning(prop, schema, def, path, name)])
      }
      if (isObj(prop) && Object.keys(prop.properties).length) rows(prop, schema, def, `${path}.`, out, seen)
      for (const alt of prop.oneOf ?? []) if (isObj(alt)) rows(alt, schema, def, `${path}.`, out, seen)
      if (isObj(prop.items)) rows(prop.items, schema, def, `${path}[].`, out, seen)
      for (const alt of prop.items?.oneOf ?? []) if (isObj(alt)) rows(alt, schema, def, `${path}[].`, out, seen)
    }
    return out
  }

  function section(schema, def, node, title) {
    const s = { label: `${schema}-${def}`, title, intro: M[`TYPE:${schema}:${def}`] ?? (missing.add(`TYPE:${schema}:${def}`), ''), notes: [], tables: [] }
    if (isObj(node) && Object.keys(node.properties).length) {
      if (node.additionalProperties === false) s.notes.push(W.closed)
      const alts = node.oneOf ?? node.anyOf
      if (alts?.every(a => a.required && !a.properties)) s.notes.push(W.requiredOne(alts.map(a => a.required.map(code).join(' + ')).join(', ')))
      s.tables.push({ rows: rows(node, schema, def) })
    } else if (node.oneOf?.every(isObj)) {
      node.oneOf.forEach((alt, i) => {
        const fixed = Object.entries(alt.properties).find(([, p]) => 'const' in p)
        s.tables.push({ caption: `*${W.variant} ${i + 1}*${fixed ? ` (${code(fixed[0])} = ${code(fixed[1].const)})` : ''}:`, rows: rows(alt, schema, def) })
      })
    } else {
      const key = `${schema}:${def}`
      let value = describe(node, schema, key)
      if (node.enum) {
        const m = node.enum.filter(v => M[valueKey(key, v)] != null)
        value = `${W.oneOf} ${node.enum.map(code).join(', ')}${m.length ? `:${m.map(v => `${BR}${code(v)} ${M[valueKey(key, v)]}`).join('')}` : ''}`
        if (m.length) for (const v of node.enum) if (M[valueKey(key, v)] == null) missing.add(valueKey(key, v))
      }
      s.value = value
    }
    return s
  }

  const parts = PARTS.map(([name, file, kind, rootTitle]) => {
    const s = schemas[name]
    const heading = name === 'rail-across' ? `${W.file}-1.0` : `${W[kind]} ${name} ${versionOf(file)}`
    return { heading, sections: [section(name, 'root', s, W[rootTitle]), ...Object.entries(s.$defs ?? {}).map(([def, node]) => section(name, def, node, def))] }
  })
  return { parts, missing, intro: W.intro(PARTS.map(([, file]) => code(file)).join(', ')) }
}

// ── Typst ───────────────────────────────────────────────────────────────────

/** Text as Typst markup: `code` stays code; the characters Typst reads are escaped; links and breaks set. */
function typ(text) {
  const out = String(text ?? '').split(/(`[^`]*`|\u27e6[^\u27e7]*\u27e7)/).map((part, i) => {
    if (!(i % 2)) return part.replace(/[\\#[\]$@<>~*_]/g, (c) => `\\${c}`).replaceAll(BR, ' #linebreak() ')
    if (part.startsWith('`')) return part
    const [label, name] = part.slice(1, -1).split('|')
    return `#link(<${label}>)[\`${name}\`]`
  }).join('')
  // A semicolon right after a link would end its code expression and vanish.
  return out.replace(/(#link\(<[^>]+>\)\[`[^`]*`\]);/g, '$1\\;')
}
function appendixTyp(lang, a) {
  const W = WORDS[lang]
  const table = (rows) => [
    '#table(columns: (24%, 11%, 1fr, 1.25fr), fill: (_, y) => if y == 0 { luma(232) },',
    `  ${W.cols.map(c => `[${c}]`).join(', ')},`,
    ...rows.map(cells => `  ${cells.map(c => `[${typ(c)}]`).join(', ')},`),
    ')',
  ].join('\n')
  const out = [`= ${W.appendix} <appendix>`, '', typ(a.intro), '']
  for (const p of a.parts) {
    out.push(`== ${p.heading}`, '')
    for (const s of p.sections) {
      out.push(`=== ${s.title} <${s.label}>`, '')
      if (s.intro) out.push(typ(s.intro), '')
      for (const n of s.notes) out.push(n.startsWith('_') ? n : typ(n), '')
      for (const t of s.tables) { if (t.caption) out.push(typ(t.caption).replace(/^\\\*(.*?)\\\*/, '*$1*'), ''); out.push(table(t.rows), '') }
      if (s.value != null) out.push(`${W.value} ${typ(s.value)}`, '')
    }
  }
  return out.join('\n')
}

function documentTyp(lang, main, a) {
  const W = WORDS[lang]
  return `${preambleTyp(W, SOURCE(lang))}

${mainTyp(main)}

#pagebreak()
${appendixTyp(lang, a)}`.replace(/\n{3,}/g, '\n\n').trimEnd() + '\n'
}

// ── Markdown ────────────────────────────────────────────────────────────────

function md(text) {
  return String(text ?? '').replaceAll(SOFT, '').split(/(`[^`]*`|\u27e6[^\u27e7]*\u27e7)/).map((part, i) => {
    if (!(i % 2)) return part.replaceAll('|', '\\|').replace(/([*_<>])/g, '\\$1').replaceAll(BR, '<br>')
    if (part.startsWith('`')) return part.replaceAll('|', '\\|')
    const [label, name] = part.slice(1, -1).split('|')
    return `[\`${name}\`](#${label})`
  }).join('')
}

function appendixMd(lang, a) {
  const W = WORDS[lang]
  const table = (rows) => [`| ${W.cols.join(' | ')} |`, '|---|---|---|---|', ...rows.map(r => `| ${r.map(md).join(' | ')} |`)].join('\n')
  const out = ['<a id="appendix"></a>', '', `## ${W.appendix}`, '', md(a.intro), '']
  for (const p of a.parts) {
    out.push(`### ${p.heading}`, '')
    for (const s of p.sections) {
      out.push(`<a id="${s.label}"></a>`, '', `#### ${s.title}`, '')
      if (s.intro) out.push(md(s.intro), '')
      for (const n of s.notes) out.push(n.startsWith('_') ? n : md(n), '')
      for (const t of s.tables) { if (t.caption) out.push(md(t.caption).replace(/^\\\*(.*?)\\\*/, '**$1**'), ''); out.push(table(t.rows), '') }
      if (s.value != null) out.push(`${W.value} ${md(s.value)}`, '')
    }
  }
  return out.join('\n')
}

function documentMd(lang, main, a) {
  const W = WORDS[lang]
  return `${mainMd(W, SOURCE(lang), main)}

${appendixMd(lang, a)}`.replace(/\n{3,}/g, '\n\n').trimEnd() + '\n'
}

// ── Write or check ──────────────────────────────────────────────────────────

const files = {}
let missing = 0
for (const [lang, base] of [['en', 'rail-across-1.0'], ['de', 'rail-across-1.0.de']]) {
  const main = readFileSync(join(HERE, 'text', `${lang}.md`), 'utf8')
  const a = appendixOf(lang)
  files[`${base}.typ`] = documentTyp(lang, main, a)
  files[`${base}.md`] = documentMd(lang, main, a)
  if (a.missing.size) {
    missing += a.missing.size
    console.error(`${lang}: ${WORDS[lang].missing} (${a.missing.size}):\n  ${[...a.missing].join('\n  ')}`)
  }
}
const check = process.argv[2] === '--check'
const stale = writeOrCheck(HERE, 'docs/format', 'docs/format/build.mjs', files, check)
process.exitCode = stale || (check && missing) ? 1 : 0
