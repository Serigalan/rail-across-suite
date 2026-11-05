import { describe, it, expect } from 'vitest'
import { SECTION_COLORS, sectionColors } from './sectionColors.js'

describe('the colours of the track sections', () => {
  it('never gives a section the colour of one beside it', () => {
    // A row of sections a–b–c–… and a switch's section w meeting three.
    const sections = Array.from({ length: 14 }, (_, i) => ({ id: `s${i}`, boundedBy: [`p${i}`, `p${i + 1}`] }))
    sections.push({ id: 'w', boundedBy: ['p3', 'p7', 'p11'] })
    const colors = sectionColors(sections)
    for (const s of sections) {
      for (const o of sections) {
        if (o !== s && o.boundedBy.some(p => s.boundedBy.includes(p))) expect(colors.get(o.id)).not.toBe(colors.get(s.id))
      }
    }
    expect(new Set(colors.values()).size).toBe(SECTION_COLORS.length)
  })
})
