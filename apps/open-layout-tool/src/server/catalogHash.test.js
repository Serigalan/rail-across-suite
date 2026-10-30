import { describe, it, expect } from 'vitest'
import { readFileSync, readdirSync } from 'node:fs'
import { join } from 'node:path'
import { catalogFiles, hashCatalogFiles } from '@rail-across/core/rules'
import { bundledCatalogFiles, bundledCatalogHash } from './catalogHash'

const DIR = join(import.meta.dirname, '..', '..', '..', '..', 'packages', 'core', 'constraints')
const names = () => readdirSync(DIR).filter(n => n.endsWith('.json')).sort()

describe('the catalogues this bundle carries', () => {
  it('are every catalogue file of the core, in name order', () => {
    expect(bundledCatalogFiles().map(f => f.name)).toEqual(names())
  })

  it('hash as the files on disk do', async () => {
    const raw = Object.fromEntries(names().map(n => [n, readFileSync(join(DIR, n), 'utf8')]))
    expect(await bundledCatalogHash()).toBe(await hashCatalogFiles(catalogFiles(raw)))
  })
})
