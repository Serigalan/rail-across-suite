import { useState } from 'react'
import { Button, Option, Options, SelectField, TextField } from '@rail-across/ui'
import { Panel } from './draft.jsx'
import { labelOf, newId } from '../model/domain.js'
import { POINT_KIND, SECTION_KIND, optionsOf } from '../labels.js'
import { DetectionIcon, SectionIcon } from '../icons.jsx'
import { Created } from './common.jsx'
import { kmNameAt, locationFrom, pickingPoints } from '../model/places.js'
import { useFields, useSetting } from './useSetting.js'

/**
 * Creating train detection: a detection point — an axle counter, a joint —
 * set on the plan while its page is open, as a preview till it is committed
 * (useSetting), named by its kilometrage (6344-5+231, the next metre free)
 * unless it is given a name; a track section, of a
 * kind, whose points are then clicked on the plan — it opens in its editor
 * (Bearbeiten) for that.
 */
export default function DetectionPanel({ domain, net, record, ops, select, mode, setMode, pending, t, language }) {
  const [page, setPage] = useState('menu')
  const [sectionKind, setSectionKind] = useState('axleCounting')
  const [name, setName] = useState('')
  const { made, setMade, offer, remake } = useSetting({
    collection: 'detectionPoints', active: page === 'point' && net.tracks.length > 0,
    mode, setMode, ops, pending, hints: ['place_point', 'place_point_move'], t,
  })
  const go = (next) => { setMade(null); setName(''); setPage(next) }
  const back = { label: t('back'), onClick: () => go('menu') }
  const points = domain.collections.detectionPoints

  if (page === 'point') {
    return (
      <Panel title={t('create_point')} intro={t('create_point_intro')} back={back}>
        <PointPage points={points} net={net} kmLines={record?.kmLines} offer={offer} remake={remake} t={t} language={language} />
        <Created what={made?.what} t={t} onEdit={() => select({ collection: 'detectionPoints', id: made.id })} />
      </Panel>
    )
  }

  if (page === 'section') {
    return (
      <Panel title={t('create_section')} intro={t('create_section_intro')} back={back}>
        <div className="ui-fields">
          <SelectField compact wide label={t('kind')} value={sectionKind} options={optionsOf(SECTION_KIND, language)} onChange={(v) => setSectionKind(v ?? 'axleCounting')} />
          <TextField compact wide label={t('name')} value={name} onChange={setName} />
        </div>
        {!points.length && <p className="ui-muted">{t('create_section_no_points')}</p>}
        <div className="ui-row">
          <Button variant="primary" disabled={!points.length}
            onClick={() => {
              const s = { id: newId(), name: name || undefined, kind: sectionKind, boundedBy: [] }
              ops.add('trackSections', s)
              select({ collection: 'trackSections', id: s.id })
              setMode(pickingPoints(ops, s.id))
            }}>
            {t('create_and_pick')}
          </Button>
        </div>
      </Panel>
    )
  }

  return (
    <Panel title={t('section_detection')} intro={t('section_detection_intro')}>
      <Options>
        <Option icon={<DetectionIcon />} onClick={() => go('point')}>{t('create_point')}</Option>
        <Option icon={<SectionIcon />} onClick={() => go('section')}>{t('create_section')}</Option>
      </Options>
    </Panel>
  )
}

/** What a detection point set is called, and the way to its editor. */
const pointMade = (t) => (p) => ({ collection: 'detectionPoints', id: p.id, what: labelOf('detectionPoints', p, t) })

/**
 * A detection point's kind and name; without a name it takes its
 * kilometrage — a metre on where another point has that one, the preview
 * itself (`self`) not counted.
 */
function PointPage({ points, net, kmLines, offer, remake, t, language }) {
  const makeOf = ({ kind, name }) => (hit, self) => {
    const at = locationFrom(hit, { directed: false })
    const taken = new Set(points.filter(p => p.id !== self).map(p => p.name))
    return { id: newId(), name: name || kmNameAt(net, kmLines, at, taken) || undefined, kind, at }
  }
  const [f, set] = useFields({ kind: 'axleCounter', name: '' }, remake, makeOf)
  offer(makeOf(f), pointMade(t))
  return <>
    <div className="ui-fields">
      <SelectField compact label={t('kind')} value={f.kind} options={optionsOf(POINT_KIND, language)} onChange={(v) => set({ kind: v ?? 'axleCounter' })} />
      <TextField compact label={t('name')} value={f.name} onChange={(name) => set({ name })} />
    </div>
    <p className="ui-muted">{t(kmLines?.length ? 'point_km_name' : 'point_km_none')}</p>
    {!net.tracks.length && <p className="ui-muted">{t('empty_net')}</p>}
  </>
}
