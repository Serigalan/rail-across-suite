import { describe, it, expect } from 'vitest'
import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

describe('the package under plain Node', () => {
  it('loads without a DOM, the bundler or a module hook — as the project server runs it', () => {
    const root = fileURLToPath(new URL('..', import.meta.url))
    const script = `
      const pc = await import('./src/index.js')
      console.log(JSON.stringify([typeof window, typeof pc.scanClearance, pc.LEVELS.length]))`
    const out = spawnSync(process.execPath, ['--input-type=module', '-e', script], { cwd: root, encoding: 'utf8' })
    expect(out.stderr).toBe('')
    const [win, scan, levels] = JSON.parse(out.stdout.trim())
    expect([win, scan]).toEqual(['undefined', 'function'])
    expect(levels).toBeGreaterThan(0)
  })
})
