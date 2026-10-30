import { describe, it, expect } from 'vitest'
import { createHash, randomBytes } from 'node:crypto'
import { canonicalJson, contentHash, derivedId } from './canonical.js'
import { sha256Hex, sha256OfText } from './sha256.js'

describe('sha256', () => {
  it('is the SHA-256 node:crypto computes, across the block boundaries', () => {
    for (const n of [0, 1, 55, 56, 63, 64, 65, 119, 120, 1000, 65537]) {
      const bytes = randomBytes(n)
      expect(sha256Hex(new Uint8Array(bytes)), String(n)).toBe(createHash('sha256').update(bytes).digest('hex'))
    }
    expect(sha256OfText('abc')).toBe('ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad')
  })
})

describe('canonical JSON (RFC 8785)', () => {
  it('writes numbers as ECMAScript does (the RFC\'s examples)', () => {
    const text = canonicalJson(JSON.parse('{"numbers":[333333333.33333329,1E30,4.50,2e-3,0.000000000000000000000000001]}'))
    expect(text).toBe('{"numbers":[333333333.3333333,1e+30,4.5,0.002,1e-27]}')
    expect(canonicalJson([100.0, 1e-7, -0])).toBe('[100,1e-7,0]')
  })

  it('sorts members by the UTF-16 code units of their names (the RFC\'s example)', () => {
    const value = JSON.parse('{"\\u20ac":"Euro Sign","\\r":"Carriage Return","\\ufb33":"Hebrew Letter Dalet With Dagesh",'
      + '"1":"One","\\ud83d\\ude00":"Emoji: Grinning Face","\\u0080":"Control","\\u00f6":"Latin Small Letter O With Diaeresis"}')
    // Read off the text: an object would put the member "1" first, whatever the text says.
    const text = canonicalJson(value)
    const at = ['\\r', '1', '\u0080', '\u00f6', '\u20ac', '\ud83d\ude00', '\ufb33'].map(k => text.indexOf(`"${k}":`))
    expect(at.every(i => i > 0)).toBe(true)
    expect([...at].sort((x, y) => x - y)).toEqual(at)
  })

  it('leaves out members that are undefined, as JSON does', () => {
    expect(canonicalJson({ b: undefined, a: [undefined, 1] })).toBe('{"a":[null,1]}')
  })

  it('refuses what JSON cannot hold', () => {
    expect(() => canonicalJson(NaN)).toThrow()
    expect(() => canonicalJson({ f: () => 0 })).toThrow()
  })

  it('hashes the canonical text: the same value the same hash, however it is written', () => {
    expect(contentHash({ a: 1, b: [1.0, 2] })).toBe(contentHash(JSON.parse('{ "b": [1, 2.0], "a": 1e0 }')))
    expect(contentHash({ a: 1 })).toMatch(/^sha256:[0-9a-f]{64}$/)
  })

  it('draws ids of UUID form from a text, the same on every device', () => {
    const id = derivedId('p\u0000stage\u0000final')
    expect(id).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-8[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/)
    expect(derivedId('p\u0000stage\u0000final')).toBe(id)
    expect(derivedId('q\u0000stage\u0000final')).not.toBe(id)
  })
})
