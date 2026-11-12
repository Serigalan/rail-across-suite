import { describe, expect, it } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import { busiestHour, minutes, percent, periodText, resourceLabel } from './study.js'
import { linkedRequest, projectsUrl } from './suite.js'
import Diagram from './Diagram.jsx'
import { textsFor } from './i18n.js'

describe('the Kapazitäts-Sim\'s figures as it shows them', () => {
  it('says rates, durations and periods as one reads them', () => {
    expect(percent(1436 / 3600)).toBe('39,9 %')
    expect(percent(null)).toBe('—')
    expect(minutes(283.5)).toBe('4:44')
    expect(periodText({ from: 8 * 3600, to: 9 * 3600 })).toBe('08:00–09:00')
  })

  it('starts on the hour the trains are thickest in', () => {
    expect(busiestHour([{ departure: 6.5 * 3600 }, { departure: 8.1 * 3600 }, { departure: 8.7 * 3600 }])).toEqual({ from: 8 * 3600, to: 9 * 3600 })
    expect(busiestHour([])).toEqual({ from: 6 * 3600, to: 7 * 3600 })
  })

  it('names a resource by its track or section', () => {
    const { t } = textsFor('de')
    expect(resourceLabel('track:t1:0-1500', () => 'Gleis 1', t)).toBe('Gleis Gleis 1, 0–1500 m')
    expect(resourceLabel('section:abcdef123456', () => '', t)).toBe('Abschnitt abcdef12')
  })

  it('reads the project and variant from its address, and leads back to its tile', () => {
    expect(linkedRequest('?project=p&variant=v')).toEqual({ project: 'p', variant: 'v' })
    expect(linkedRequest('')).toBeNull()
    expect(projectsUrl()).toBe('../#/capacity-sim')
  })

  it('draws the blocking time stairs, compressed or as timetabled', () => {
    const analysis = {
      trains: [{ id: 'a', shift: 0 }, { id: 'b', shift: 100 }],
      input: [
        { id: 'a', kind: 'RE', blocks: [{ begin: 0, end: 100, span: [0, 0.5] }, { begin: 50, end: 200, span: [0.4, 1] }] },
        { id: 'b', kind: 'Gz', blocks: [{ begin: 300, end: 450, span: [0, 0.5] }] },
      ],
    }
    const svg = renderToStaticMarkup(<Diagram analysis={analysis} compressed fromName="X" toName="Y" />)
    expect(svg.match(/<rect/g)).toHaveLength(3)
    expect(svg).toContain('>X<')
  })
})
