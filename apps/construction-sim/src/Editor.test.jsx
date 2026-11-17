import { describe, expect, it } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import { stageLabel, timeline, trackState } from './model/stages.js'
import { linkedRequest, projectsUrl } from './suite.js'
import { I18n, textsFor } from './i18n.js'
import Editor from './Editor.jsx'
import { changedClosure, closureLabel, closureWhen, diversionOf, firstDayOf, isCancelled, newClosure, newScenario, toggledCancel, withClosure, withDiversion, withoutClosure, withoutScenario, withScenario } from './model/scenario.js'

const record = {
  id: 'p', platforms: [],
  tracks: [
    { id: 'a', name: 'A' }, { id: 'b', name: 'B', lifespan: { to: '2028-06-01' } },
    { id: 'c', name: 'C', lifespan: { from: '2028-06-02' } },
  ],
  switches: [{ switchId: 'v', kind: 'stage_link', ports: { A: { trackId: 'a', endpoint: 'END' }, B1: { trackId: 'b', endpoint: 'BEGIN' }, B2: { trackId: 'c', endpoint: 'BEGIN' } } }],
}

describe('the Construction-Sim (decision 0007)', () => {
  it('lays out the stages and what each changes', () => {
    const stages = timeline(record)
    expect(stages.map(s => s.key)).toEqual(['existing', '2028-06-02', 'final'])
    expect([...stages[0].tracks]).toEqual(['a', 'b'])
    expect(stages[1].came.tracks).toEqual(['c'])
    expect(stages[1].went.tracks).toEqual(['b'])
    expect(trackState(stages, 0, 'b')).toBe('goes')
    expect(trackState(stages, 1, 'c')).toBe('came')
    expect(trackState(stages, 1, 'a')).toBe('stays')
    expect(trackState(stages, 1, 'b')).toBeNull()
    const { t } = textsFor('de')
    expect(stageLabel(stages[1], t)).toBe('ab 02.06.2028')
  })

  it('reads the project and variant from its address, and leads back to its tile', () => {
    expect(linkedRequest('?project=p&variant=v')).toEqual({ project: 'p', variant: 'v' })
    expect(projectsUrl()).toBe('../#/construction-sim')
  })

  it('lists the stages and what the first changes', () => {
    const html = renderToStaticMarkup(
      <I18n.Provider value={textsFor('de')}>
        <Editor project={{ title: 'Umbau' }} variant={{ name: 'Bestand' }} revision={{ number: 3 }} record={record} local={false} />
      </I18n.Provider>,
    )
    expect(html).toContain('ab 02.06.2028')
    expect(html).toContain('Endzustand')
    expect(html).toContain('Umbau')
  })
})

describe('a construction scenario as the Baubetriebs-Sim changes it (decision 0014)', () => {
  it('closes a stretch of a track on a day, all day or in a possession, and names it', () => {
    let sc = newScenario('S')
    const c = newClosure({ trackId: 't1', from: 412.34, to: 100 }, '2027-05-03')
    expect(c).toMatchObject({ at: { anchor: 'station', trackId: 't1', startStation: 100, endStation: 412.3 }, firstDay: '2027-05-03', lastDay: '2027-05-03' })
    sc = withClosure(sc, c)
    sc = changedClosure(sc, c.id, { window: { from: 79200, to: 18000 }, lastDay: '2027-05-07' })
    const { t } = textsFor('de')
    expect(closureLabel(sc.closures[0], () => 'Gl. 1')).toBe('Gl. 1 · 100–412 m')
    expect(closureWhen(sc.closures[0], t)).toBe('2027-05-03 – 2027-05-07, 22:00–05:00')
    expect(closureWhen(changedClosure(sc, c.id, { window: null }).closures[0], t)).toBe('2027-05-03 – 2027-05-07, ganztägig')
    expect(firstDayOf(sc, 'x')).toBe('2027-05-03')
    expect(firstDayOf(newScenario('E'), 'x')).toBe('x')
    expect(withoutClosure(sc, c.id).closures).toEqual([])
  })

  it('takes trains out and puts them back, diverts them by hand, and is kept in its timetable\'s data', () => {
    let sc = toggledCancel(newScenario('S'), 'a')
    expect(isCancelled(sc, 'a')).toBe(true)
    expect(isCancelled(toggledCancel(sc, 'a'), 'a')).toBe(false)
    sc = withDiversion(sc, 'b', [{ id: 'p1' }, { id: 'p2' }])
    expect(diversionOf(sc, 'b').points).toHaveLength(2)
    expect(diversionOf(withDiversion(sc, 'b', null), 'b')).toBeNull()
    const data = withScenario({ trains: [] }, sc)
    expect(withScenario(data, { ...sc, name: 'T' }).scenarios.map(s => s.name)).toEqual(['T'])
    expect(withoutScenario(data, sc.id).scenarios).toEqual([])
  })
})
