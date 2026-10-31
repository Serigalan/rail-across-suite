import { useState } from 'react'
import { Button, Option, Options, Panel, SelectField, TextField } from '@rail-across/ui'
import { labelOf, newId } from '../model/domain.js'
import { POINT_KIND, SECTION_KIND, optionsOf } from '../labels.js'
import { DetectionIcon, SectionIcon } from '../icons.jsx'
import { Created } from './common.jsx'
import { locationFrom, pickingPoints, placing } from '../model/places.js'

/**
 * Creating train detection: a detection point — an axle counter, a joint —
 * set on the plan, its page staying for the next one; a track section, of a
 * kind, whose points are then clicked on the plan — it opens in its editor
 * (Bearbeiten) for that.
 */
export default function DetectionPanel({ domain, net, ops, select, setMode, t, language }) {
  const [page, setPage] = useState('menu')
  const [made, setMade] = useState(null)
  const [pointKind, setPointKind] = useState('axleCounter')
  const [sectionKind, setSectionKind] = useState('axleCounting')
  const [name, setName] = useState('')
  const go = (next) => { setMade(null); setName(''); setPage(next) }
  const back = { label: t('back'), onClick: () => go('menu') }
  const points = domain.collections.detectionPoints

  if (page === 'point') {
    return (
      <Panel title={t('create_point')} intro={t('create_point_intro')} back={back}>
        <div className="ui-fields">
          <SelectField compact label={t('kind')} value={pointKind} options={optionsOf(POINT_KIND, language)} onChange={(v) => setPointKind(v ?? 'axleCounter')} />
          <TextField compact label={t('name')} value={name} onChange={setName} />
        </div>
        <div className="ui-row">
          <Button variant="primary" disabled={!net.tracks.length}
            onClick={() => setMode(placing('detectionPoints', (hit) => ({ id: newId(), name: name || undefined, kind: pointKind, at: locationFrom(hit, { directed: false }) }), (p) => {
              ops.add('detectionPoints', p)
              setMade({ id: p.id, what: labelOf('detectionPoints', p, t) })
            }))}>
            {t('place_on_map')}
          </Button>
        </div>
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
