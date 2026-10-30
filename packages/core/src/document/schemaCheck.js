// The domain schemas checked as JSON Schema — the part of it they use, no more,
// so that the Python core reads them exactly alike (core-py, schema_check.py,
// held to the same vectors). A schema that uses another keyword is refused
// when it is registered rather than half understood.

/** The keywords understood; annotations (`x-…`) are ignored. */
export const KEYWORDS = new Set([
  '$schema', '$id', '$ref', '$defs', '$comment', 'title', 'description',
  'type', 'enum', 'const',
  'properties', 'required', 'additionalProperties', 'patternProperties', 'minProperties',
  'items', 'minItems', 'maxItems',
  'minimum', 'maximum', 'minLength', 'pattern',
  'oneOf', 'anyOf', 'allOf',
])

const isObject = (v) => v !== null && typeof v === 'object' && !Array.isArray(v)

/** Every keyword of `schema` this check does not understand, as "path: keyword". */
export function unknownKeywords(schema, path = '') {
  const out = []
  if (!isObject(schema)) return out
  for (const [k, v] of Object.entries(schema)) {
    if (!KEYWORDS.has(k) && !k.startsWith('x-')) out.push(`${path || '/'}: ${k}`)
    if (k === 'properties' || k === 'patternProperties' || k === '$defs') {
      for (const [name, sub] of Object.entries(v)) out.push(...unknownKeywords(sub, `${path}/${k}/${name}`))
    } else if (k === 'items' || k === 'additionalProperties') {
      out.push(...unknownKeywords(v, `${path}/${k}`))
    } else if (k === 'oneOf' || k === 'anyOf' || k === 'allOf') {
      v.forEach((sub, i) => out.push(...unknownKeywords(sub, `${path}/${k}/${i}`)))
    }
  }
  return out
}

function sameJson(a, b) {
  if (a === b) return true
  if (Array.isArray(a)) return Array.isArray(b) && a.length === b.length && a.every((x, i) => sameJson(x, b[i]))
  if (isObject(a)) {
    if (!isObject(b)) return false
    const ka = Object.keys(a), kb = Object.keys(b)
    return ka.length === kb.length && ka.every(k => Object.hasOwn(b, k) && sameJson(a[k], b[k]))
  }
  return false
}

const TYPES = {
  null: (v) => v === null,
  boolean: (v) => typeof v === 'boolean',
  number: (v) => typeof v === 'number' && Number.isFinite(v),
  integer: (v) => Number.isInteger(v),
  string: (v) => typeof v === 'string',
  array: (v) => Array.isArray(v),
  object: isObject,
}

/** The code points of a string — what JSON Schema counts as its length. */
const lengthOf = (s) => [...s].length

/**
 * A set of schemas that refer to each other by `$id` ("core#/$defs/Track").
 * `check(id, value)` gives the breaches, each `{ path, message }`, at most
 * `limit` of them; an empty list is a value the schema takes.
 */
export class SchemaSet {
  constructor(schemas = []) {
    this.byId = new Map()
    this.patterns = new Map()
    for (const s of schemas) this.add(s)
  }

  add(schema) {
    if (!isObject(schema) || typeof schema.$id !== 'string') throw new Error('a schema needs its $id')
    const unknown = unknownKeywords(schema)
    if (unknown.length) throw new Error(`schema ${schema.$id} uses what this check does not know: ${unknown.join(', ')}`)
    this.byId.set(schema.$id, schema)
  }

  /** `ref` a schema's $id ("core"), or a part of one ("core#/$defs/TrackRange"). */
  check(ref, value, { limit = 20 } = {}) {
    const [root, node] = ref.includes('#') ? this.#resolve(null, ref) : [this.byId.get(ref), this.byId.get(ref)]
    if (!root) throw new Error(`no schema ${ref}`)
    const errors = []
    this.#walk(root, node, value, '', errors, limit)
    return errors
  }

  #resolve(root, ref) {
    const hash = ref.indexOf('#')
    const target = hash > 0 ? this.byId.get(ref.slice(0, hash)) : root
    if (!target) throw new Error(`no schema ${ref.slice(0, hash)}`)
    let node = target
    for (const part of ref.slice(hash + 1).split('/').filter(Boolean)) node = node?.[part]
    if (!isObject(node)) throw new Error(`cannot resolve ${ref}`)
    return [target, node]
  }

  #regex(pattern) {
    let re = this.patterns.get(pattern)
    if (!re) { re = new RegExp(pattern, 'u'); this.patterns.set(pattern, re) }
    return re
  }

  /** Whether `value` passes `schema`, without recording why not. */
  #passes(root, schema, value, path) {
    const errors = []
    this.#walk(root, schema, value, path, errors, 1)
    return errors.length === 0
  }

  #walk(root, schema, value, path, errors, limit) {
    if (errors.length >= limit) return
    const fail = (message) => { if (errors.length < limit) errors.push({ path: path || '/', message }) }

    if (schema.$ref) {
      const [r, node] = this.#resolve(root, schema.$ref)
      this.#walk(r, node, value, path, errors, limit)
    }
    if (schema.type !== undefined) {
      const types = Array.isArray(schema.type) ? schema.type : [schema.type]
      if (!types.some(t => TYPES[t](value))) { fail(`is not ${types.join(' or ')}`); return }
    }
    if (schema.enum && !schema.enum.some(e => sameJson(e, value))) fail(`is not one of ${JSON.stringify(schema.enum)}`)
    if (Object.hasOwn(schema, 'const') && !sameJson(schema.const, value)) fail(`is not ${JSON.stringify(schema.const)}`)

    if (typeof value === 'number') {
      if (schema.minimum !== undefined && value < schema.minimum) fail(`is less than ${schema.minimum}`)
      if (schema.maximum !== undefined && value > schema.maximum) fail(`is more than ${schema.maximum}`)
    }
    if (typeof value === 'string') {
      if (schema.minLength !== undefined && lengthOf(value) < schema.minLength) fail(`is shorter than ${schema.minLength}`)
      if (schema.pattern !== undefined && !this.#regex(schema.pattern).test(value)) fail(`does not match ${schema.pattern}`)
    }
    if (Array.isArray(value)) {
      if (schema.minItems !== undefined && value.length < schema.minItems) fail(`has fewer than ${schema.minItems} items`)
      if (schema.maxItems !== undefined && value.length > schema.maxItems) fail(`has more than ${schema.maxItems} items`)
      if (schema.items) value.forEach((v, i) => this.#walk(root, schema.items, v, `${path}/${i}`, errors, limit))
    }
    if (isObject(value)) {
      const keys = Object.keys(value)
      for (const k of schema.required ?? []) if (!Object.hasOwn(value, k)) fail(`lacks ${k}`)
      if (schema.minProperties !== undefined && keys.length < schema.minProperties) fail(`has fewer than ${schema.minProperties} members`)
      const props = schema.properties ?? {}
      const patterns = Object.entries(schema.patternProperties ?? {})
      for (const k of keys) {
        const at = `${path}/${k}`
        let known = false
        if (Object.hasOwn(props, k)) { known = true; this.#walk(root, props[k], value[k], at, errors, limit) }
        for (const [p, sub] of patterns) {
          if (this.#regex(p).test(k)) { known = true; this.#walk(root, sub, value[k], at, errors, limit) }
        }
        if (!known && schema.additionalProperties !== undefined) {
          if (schema.additionalProperties === false) fail(`has ${k}, which it may not`)
          else if (isObject(schema.additionalProperties)) this.#walk(root, schema.additionalProperties, value[k], at, errors, limit)
        }
      }
    }

    if (schema.allOf) for (const sub of schema.allOf) this.#walk(root, sub, value, path, errors, limit)
    if (schema.anyOf && !schema.anyOf.some(sub => this.#passes(root, sub, value, path))) fail('matches none of anyOf')
    if (schema.oneOf) {
      const n = schema.oneOf.filter(sub => this.#passes(root, sub, value, path)).length
      if (n !== 1) {
        // Where exactly one alternative has the type, its own breaches say more.
        const typed = schema.oneOf.filter(sub => !sub.type || [sub.type].flat().some(t => TYPES[t](value)))
        if (n === 0 && typed.length === 1) this.#walk(root, typed[0], value, path, errors, limit)
        else fail(n === 0 ? 'matches none of oneOf' : 'matches more than one of oneOf')
      }
    }
  }
}
