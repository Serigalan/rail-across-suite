import { describe, it, expect } from 'vitest'
import { createHash } from 'node:crypto'
import { readFileSync, readdirSync } from 'node:fs'
import { join } from 'node:path'
import { catalogFiles, hashCatalogFiles } from './catalogHash.js'

const DIR = join(import.meta.dirname, '..', '..', 'constraints')
const onDisk = () => Object.fromEntries(readdirSync(DIR).filter(n => n.endsWith('.json'))
  .map(n => [`../../constraints/${n}`, readFileSync(join(DIR, n), 'utf8')]))

describe('catalogue hash (R0.1)', () => {
  it('takes the files in name order, by their names alone', () => {
    const names = catalogFiles({ 'x/b.json': '', 'y/a.json': '' }).map(f => f.name)
    expect(names).toEqual(['a.json', 'b.json'])
  })

  it('equals the hash the optimizer service computes over the raw file bytes', async () => {
    // The same recipe as olt_optimizer/regelwerk.py catalog_hash, here with
    // node:crypto over the files as they lie on disk.
    const h = createHash('sha256')
    for (const name of readdirSync(DIR).filter(n => n.endsWith('.json')).sort()) {
      h.update(Buffer.from(name + '\0', 'utf-8'))
      h.update(Buffer.concat([readFileSync(join(DIR, name)), Buffer.from([0])]))
    }
    expect(await hashCatalogFiles(catalogFiles(onDisk()))).toBe(h.digest('hex'))
  })

  it('changes with any byte of any file', async () => {
    const a = await hashCatalogFiles([{ name: 'a.json', text: '{}' }])
    const b = await hashCatalogFiles([{ name: 'a.json', text: '{ }' }])
    expect(a).not.toBe(b)
  })
})
