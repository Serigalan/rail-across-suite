import { describe, expect, it } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import { stageLabel, timeline, trackState } from './model/stages.js'
import { linkedRequest, projectsUrl } from './suite.js'
import { I18n, textsFor } from './i18n.js'
import Editor from './Editor.jsx'

const record = {
  id: 'p', platforms: [],
  tracks: [
    { id: 'a', name: 'A' }, { id: 'b', name: 'B', lifespan: { to: '2028-06-01' } },
    { id: 'c', name: 'C', lifespan: { from: '2028-06-02' } },
  ],
  switches: [{ switchId: 'v', kind: 'stage_link', ports: { A: { trackId: 'a', endpoint: 'END' }, B1: { trackId: 'b', endpoint: 'BEGIN' }, B2: { trackId: 'c', endpoint: 'BEGIN' } } }],
}

describe('the Construction-Sim (decision 0027)', () => {
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
