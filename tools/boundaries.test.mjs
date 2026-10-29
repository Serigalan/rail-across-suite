import { describe, it } from 'node:test'
import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { breach, checkBoundaries, importsOf, TRANSITIONAL } from './boundaries.mjs'

/** A repository in a temporary directory: { 'apps/a/x.js': '…' }. */
function repo(files) {
  const root = mkdtempSync(join(tmpdir(), 'ord-boundaries-'))
  for (const [path, text] of Object.entries(files)) {
    mkdirSync(dirname(join(root, path)), { recursive: true })
    writeFileSync(join(root, path), text)
  }
  return root
}

describe('importsOf', () => {
  it('finds static, re-exported, dynamic and required imports with their lines', () => {
    const text = "import a from './a'\nexport { b } from \"../b\"\nconst c = await import('c')\nconst d = require('d')\nimport './e'\n"
    assert.deepEqual(importsOf(text).map(i => [i.spec, i.line]), [['./a', 1], ['../b', 2], ['c', 3], ['d', 4], ['./e', 5]])
  })
  it('skips imports in comments', () => {
    assert.deepEqual(importsOf("// import x from './x'\n/* import y from './y' */\nconst u = 'http://host'\n"), [])
  })
})

describe('breach', () => {
  it('allows apps to import packages and anything to import within its own area', () => {
    assert.equal(breach('app:open-layout-tool', 'package:core'), null)
    assert.equal(breach('app:open-layout-tool', 'app:open-layout-tool'), null)
    assert.equal(breach('server', 'package:core'), null)
  })
  it('refuses an app importing another app, and anything else importing an app', () => {
    assert.equal(breach('app:open-signalling-tool', 'app:open-layout-tool'), 'an app imports another app')
    assert.equal(breach('package:core', 'app:open-layout-tool'), 'package imports an app')
    assert.equal(breach('server', 'app:open-layout-tool'), 'server imports an app')
    assert.equal(breach('service:optimizer', 'app:open-layout-tool'), 'service imports an app')
  })
})

describe('checkBoundaries', () => {
  it('reports imports across the boundaries, by path and by package name', () => {
    const root = repo({
      'apps/open-layout-tool/package.json': JSON.stringify({ name: 'open-layout-tool' }),
      'apps/open-layout-tool/src/a.js': "import { core } from '@rail-across/core'\nimport b from './b.js'\n",
      'apps/open-signalling-tool/src/m.js': "import x from '../../open-layout-tool/src/a.js'\nimport y from 'open-layout-tool/src/a.js'\n",
      'packages/core/package.json': JSON.stringify({ name: '@rail-across/core' }),
      'packages/core/index.js': "import z from '../../apps/open-layout-tool/src/a.js'\nimport { readFileSync } from 'node:fs'\n",
      'server/api.js': "import { core } from '@rail-across/core'\n",
    })
    try {
      const found = checkBoundaries(root).map(f => `${f.file}:${f.line} ${f.rule}`).sort()
      assert.deepEqual(found, [
        'apps/open-signalling-tool/src/m.js:1 an app imports another app',
        'apps/open-signalling-tool/src/m.js:2 an app imports another app',
        'packages/core/index.js:1 package imports an app',
      ])
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  })
  it('is quiet for a repository that keeps them', () => {
    const root = repo({ 'apps/a/x.js': "import y from './y.js'\n", 'packages/core/i.js': 'export const k = 1\n' })
    try {
      assert.deepEqual(checkBoundaries(root), [])
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  })
})

describe('TRANSITIONAL', () => {
  it('allows only the project server into the Open-Layout-Tool, until phase 1', () => {
    assert.deepEqual([...TRANSITIONAL], ['server → app:open-layout-tool'])
  })
})
