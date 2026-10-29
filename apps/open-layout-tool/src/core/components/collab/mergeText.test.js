import { describe, it, expect } from 'vitest'
import { translations } from '../../locales/i18n'
import { conflictText, valueText } from './mergeText'

const t = (key) => translations.en[key] ?? key

describe('a platform\'s location in a conflict', () => {
  const c = { kind: 'field', collection: 'platforms', objectId: 'P', field: 'at', label: 'Halle-Trotha' }

  it('names the location as what the two sides differ in', () => {
    expect(conflictText(t, c, { mineLabel: 'mine', theirsLabel: 'theirs' })).toContain(translations.en.field_at)
  })

  it('shows each side\'s stations and side', () => {
    expect(valueText(t, c, { trackId: 'T2', startStation: 35, endStation: 175, side: 'left' }))
      .toBe(`35–175 m, ${translations.en.switch_side_left}`)
    expect(valueText(t, c, { trackId: 'T2', startStation: 20.04, endStation: 160, side: 'right' }))
      .toBe(`20–160 m, ${translations.en.switch_side_right}`)
  })
})
