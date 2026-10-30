import { describe, it, expect } from 'vitest'
import { SchemaSet, unknownKeywords } from './schemaCheck.js'
import { schemaOf } from './door.js'

describe('the schema check', () => {
  it('understands every keyword the domain schemas use', () => {
    for (const name of ['rail-across', 'core', 'layout']) expect(unknownKeywords(schemaOf(name)), name).toEqual([])
  })

  it('refuses a schema with a keyword it does not know, rather than half understanding it', () => {
    expect(() => new SchemaSet([{ $id: 'x', type: 'object', if: { required: ['a'] } }])).toThrow(/if/)
    expect(() => new SchemaSet([{ type: 'object' }])).toThrow(/\$id/)
  })

  it('names where a value breaks the schema, and what', () => {
    const set = new SchemaSet([{
      $id: 'x', type: 'object', required: ['a'], additionalProperties: false,
      properties: { a: { type: 'array', items: { $ref: '#/$defs/N' } } },
      $defs: { N: { type: 'number', minimum: 0 } },
    }])
    expect(set.check('x', { a: [1, 2] })).toEqual([])
    expect(set.check('x', { a: [1, -2, 'x'], b: 1 })).toEqual([
      { path: '/a/1', message: 'is less than 0' },
      { path: '/a/2', message: 'is not number' },
      { path: '/', message: 'has b, which it may not' },
    ])
    expect(set.check('x#/$defs/N', -1)).toEqual([{ path: '/', message: 'is less than 0' }])
  })

  it('follows references into another schema of the set by its $id', () => {
    const set = new SchemaSet(['rail-across', 'core', 'layout'].map(schemaOf))
    expect(set.check('layout#/$defs/NamedTrackPath', { id: 'r', path: { trackIds: ['t'] } })).toEqual([])
    expect(set.check('layout#/$defs/NamedTrackPath', { id: 'r', path: { trackIds: [1] } }))
      .toEqual([{ path: '/path/trackIds/0', message: 'is not string' }])
  })
})
