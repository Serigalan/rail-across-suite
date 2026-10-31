import { Badge, GroupedList, Panel } from '@rail-across/ui'
import { protectionSystemOf, titleIn } from '@rail-across/core/signalling'
import { findLogical, hasScreen, labelOf } from '../model/domain.js'
import { POINT_KIND, SECTION_KIND, VIRTUAL_FUNCTION, baliseWord, logicalSignalWord, word } from '../labels.js'
import { signalTone } from '../plan/symbols.js'
import { OFFSET, locationFrom, placing, trackName } from '../model/places.js'
import LocationEditor from './editors/LocationEditor.jsx'
import LogicalEditor from './editors/LogicalEditor.jsx'
import { PointEditor, SectionEditor } from './editors/DetectionEditors.jsx'
import BaliseEditor from './editors/BaliseEditor.jsx'

/** The groups of signal masts in the overview, in their order. */
const SIGNAL_GROUPS = ['main', 'distant', 'shunting', 'board', 'bare', 'virtual']

/** The group a signal mast is listed under: by what its logical signals are, a board, a mast with nothing yet, a virtual signal. */
function signalGroup(s) {
  if (s.kind === 'virtual') return 'virtual'
  if (!(s.logicalSignals ?? []).length && !hasScreen(s)) return (s.frames ?? []).length ? 'board' : 'bare'
  const tone = signalTone(s)
  return tone === 'marker' ? 'board' : tone
}

/**
 * Editing what the three panels before it create — signals, train detection,
 * balises and magnets: an overview of them all under their kinds, each group
 * folding away, a search over all (as the Layout-Tool lists a project's
 * tracks), and a click on one, in the overview or on the plan, opens it in
 * its editor; a logical signal is opened from its mast. The way back leads to
 * the overview, from a logical signal to its mast.
 */
export default function EditPanel({ domain, net, findings, extents, selection, select, ops, mode, setMode, readOnly, t, language }) {
  const { signals, detectionPoints, trackSections, balises } = domain.collections
  const toOverview = { label: t('back'), onClick: () => select(null) }
  const editorProps = { domain, net, findings, extents, ops, mode, setMode, select, readOnly, t, language }
  const find = (name) => domain.collections[name].find(o => o.id === selection?.id)

  switch (selection?.collection) {
    case 'signals': {
      const signal = find('signals')
      if (!signal) break
      return (
        <Panel title={t(signal.kind === 'virtual' ? 'object_virtual' : 'object_signals')} back={toOverview}>
          <LocationEditor key={signal.id} signal={signal} {...editorProps}
            onChange={(s) => ops.update('signals', s)} onDelete={() => ops.remove('signals', signal.id)}
            onPlace={() => setMode(placing('signals', (hit) => ({ ...signal, at: locationFrom(hit, { offset: signal.at?.offset ?? OFFSET.signals }) }), (s) => ops.update('signals', s)))} />
        </Panel>
      )
    }
    case 'logicalSignals': {
      const found = findLogical(domain, selection.id)
      if (!found.signal) break
      return (
        <Panel title={t('logical_signal')} back={{ label: labelOf('signals', found.signal, t), onClick: () => select({ collection: 'signals', id: found.signal.id }) }}>
          <LogicalEditor key={selection.id} id={selection.id} found={found} {...editorProps} />
        </Panel>
      )
    }
    case 'detectionPoints': {
      const point = find('detectionPoints')
      if (!point) break
      return <Panel title={t('object_detectionPoints')} back={toOverview}><PointEditor key={point.id} point={point} {...editorProps} /></Panel>
    }
    case 'trackSections': {
      const section = find('trackSections')
      if (!section) break
      return <Panel title={t('object_trackSections')} back={toOverview}><SectionEditor key={section.id} section={section} {...editorProps} /></Panel>
    }
    case 'balises': {
      const balise = find('balises')
      if (!balise) break
      return <Panel title={t('object_balises')} back={toOverview}><BaliseEditor key={balise.id} balise={balise} {...editorProps} /></Panel>
    }
    default:
  }

  // ── the overview ──
  const flagged = (list, collection) => new Set(list.filter(f => f.collection === collection).map(f => f.id))
  const bad = { signals: flagged(findings.errors, 'signals'), balises: flagged(findings.errors, 'balises'), sections: flagged(findings.warnings, 'trackSections') }
  const where = (at) => (at?.anchor === 'station' ? `${trackName(net, at.trackId)} · ${Math.round(at.station)} m` : t('orphan'))
  const byName = (a, b) => String(a.label).localeCompare(String(b.label), 'de', { numeric: true })
  const danger = <Badge tone="danger">!</Badge>

  const signalItems = signals.map(s => ({
    group: signalGroup(s), id: `signals:${s.id}`, label: labelOf('signals', s, t),
    sub: [s.kind === 'virtual' ? word(VIRTUAL_FUNCTION, s.virtualFunction, language)
      : (s.logicalSignals ?? []).map(l => logicalSignalWord(l, language)).join(' + '), where(s.at)].filter(Boolean).join(' · '),
    end: bad.signals.has(s.id) ? danger : null,
  }))
  const groups = [
    ...SIGNAL_GROUPS.map(g => ({ key: `signals-${g}`, heading: t('section_signals'), title: t(`group_${g}`), items: signalItems.filter(i => i.group === g).sort(byName) })),
    {
      key: 'trackSections', heading: t('section_detection'), title: t('track_sections'),
      items: trackSections.map(s => {
        const ext = extents.get(s.id)
        const m = ext ? Math.round(ext.parts.reduce((sum, p) => sum + (p.to - p.from), 0)) : 0
        return { id: `trackSections:${s.id}`, label: labelOf('trackSections', s, t), sub: `${word(SECTION_KIND, s.kind, language)} · ${s.boundedBy.length} × · ${m} m`, end: bad.sections.has(s.id) ? <Badge tone="warn">!</Badge> : null }
      }).sort(byName),
    },
    {
      key: 'detectionPoints', heading: t('section_detection'), title: t('detection_points'),
      items: detectionPoints.map(p => ({ id: `detectionPoints:${p.id}`, label: labelOf('detectionPoints', p, t), sub: `${word(POINT_KIND, p.kind, language)} · ${where(p.at)}` })).sort(byName),
    },
    ...[...new Set(balises.map(b => b.system))].sort().map(system => ({
      key: `balises-${system}`, heading: t('section_balises'), title: titleIn(protectionSystemOf(system), language) ?? system,
      items: balises.filter(b => b.system === system).map(b => ({
        id: `balises:${b.id}`, label: [b.name, baliseWord(b, language, t)].filter(Boolean).join(' · '),
        sub: [b.signalId ? labelOf('signals', signals.find(s => s.id === b.signalId), t) : null, where(b.at)].filter(Boolean).join(' · '),
        end: bad.balises.has(b.id) ? danger : null,
      })).sort((a, b) => a.sub.localeCompare(b.sub, 'de', { numeric: true })),
    })),
  ]
  // What is chosen elsewhere — a logical signal's mast — is marked here.
  const chosen = selection?.collection === 'logicalSignals' ? `signals:${findLogical(domain, selection.id).signal?.id}` : selection ? `${selection.collection}:${selection.id}` : null

  return (
    <Panel title={t('section_edit')} intro={t('section_edit_intro')}>
      <GroupedList label={t('section_edit')} groups={groups} selected={chosen} empty={t('edit_empty')} searchLabel={t('search_objects')} noneFound={t('search_none')}
        onSelect={(key) => {
          const at = key.indexOf(':')
          select({ collection: key.slice(0, at), id: key.slice(at + 1) })
        }} />
    </Panel>
  )
}
