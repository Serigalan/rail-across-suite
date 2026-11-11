import { describe, expect, it } from 'vitest'
import { toolOf } from './tool.js'

describe('the tool a page is (decision 0025)', () => {
  it('is the Civil-Tool under /civil/ or when asked, else the Layout-Tool', () => {
    expect(toolOf({ pathname: '/civil/', search: '?project=p' })).toBe('civil')
    expect(toolOf({ pathname: '/civil/index.html', search: '' })).toBe('civil')
    expect(toolOf({ pathname: '/layout/', search: '?project=p' })).toBe('layout')
    expect(toolOf({ pathname: '/', search: '?tool=civil' })).toBe('civil')
    expect(toolOf({ pathname: '/civilisation/layout/', search: '' })).toBe('layout')
  })
})
