import { useEffect, useMemo, useState } from 'react'
import { Badge, GroupedList, Option, Options, OptionsDivider, OptionsHeading } from '@rail-across/ui'
import { Panel } from './draft.jsx'
import { protectionSystemOf, titleIn } from '@rail-across/core/signalling'
import { findLogical, hasScreen, labelOf } from '../model/domain.js'
import { POINT_KIND, SECTION_KIND, VIRTUAL_FUNCTION, baliseWord, logicalSignalWord, word } from '../labels.js'
import { signalTone } from '../plan/symbols.js'
import { OFFSET, locationFrom, mountedFrom, placing, sideAlong, trackName } from '../model/places.js'
import LocationEditor from './editors/LocationEditor.jsx'
import LogicalEditor from './editors/LogicalEditor.jsx'
import { PointEditor, SectionEditor } from './editors/DetectionEditors.jsx'
import BaliseEditor from './editors/BaliseEditor.jsx'
import { sectionColors } from '../model/sectionColors.js'
import { BaliseIcon, BoardIcon, DetectionIcon, LocationIcon, SectionIcon, SignalIcon, VirtualIcon } from '../icons.jsx'

/** The icon of each kind in the menu. */
const ICONS = {
  'signals-main': <SignalIcon />, 'signals-distant': <SignalIcon />, 'signals-shunting': <SignalIcon />, 'signals-board': <BoardIcon />,
  'signals-bare': <LocationIcon />, 'signals-virtual': <VirtualIcon />, trackSections: <SectionIcon />, detectionPoints: <DetectionIcon />,
}

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
 * balises and magnets: a menu of their kinds — main signals, distant signals,
 * …, track sections, detection points, the balises of each system —, each
 * opening the list of its objects with a search over them, and a click on
 * one, in the list or on the plan, opens it in its editor; a logical signal
 * is opened from its mast. The way back leads to the list, from a logical
 * signal to its mast. While the track sections are listed, the plan draws
 * each in a colour of its own, none in its neighbour's, and the list names
 * each in it.
 */
export default function EditPanel({ domain, net, core, findings, extents, selection, select, ops, mode, setMode, setSketch, showOnPlan, readOnly, t, language }) {
  const { signals, detectionPoints, trackSections, balises } = domain.collections
  // The kind listed, or null for the menu of them.
  const [listed, setListed] = useState(null)
  const colors = useMemo(() => sectionColors(trackSections), [trackSections])
  // The track sections listed: each on the plan in its colour, its name on it in that colour.
  const colouring = listed === 'trackSections' && !selection
  useEffect(() => {
    if (!colouring) return undefined
    const parts = trackSections.flatMap(s => (extents.get(s.id)?.parts ?? []).map(p => ({ ...p, color: colors.get(s.id), id: s.id })))
    const longest = trackSections.map(s => {
      const p = [...(extents.get(s.id)?.parts ?? [])].sort((a, b) => (b.to - b.from) - (a.to - a.from))[0]
      return p ? { ...p, text: labelOf('trackSections', s, t), color: colors.get(s.id) } : null
    }).filter(Boolean)
    setSketch?.({ stretches: parts, labels: longest })
    return () => setSketch?.(null)
  }, [colouring, trackSections, extents, colors, setSketch, t])
  const toOverview = { label: t('back'), onClick: () => select(null) }
  const editorProps = { domain, net, core, findings, extents, ops, mode, setMode, setSketch, showOnPlan, select, readOnly, t, language }
  const find = (name) => domain.collections[name].find(o => o.id === selection?.id)

  switch (selection?.collection) {
    case 'signals': {
      const signal = find('signals')
      if (!signal) break
      return (
        <Panel title={t(signal.kind === 'virtual' ? 'object_virtual' : 'object_signals')} back={toOverview}>
          <LocationEditor key={signal.id} signal={signal} {...editorProps}
            onChange={(s) => ops.update('signals', s)} onDelete={() => ops.remove('signals', signal.id)}
            onPlace={() => setMode(placing('signals', (hit) => ({
              ...signal,
              // A gantry's screen keeps where it stands to the track, seen the way it acts.
              at: signal.foundation
                ? mountedFrom(hit, { screen: { side: sideAlong(signal.at, signal.at?.side) ?? null, offset: signal.at?.offset }, foundation: signal.foundation }).at
                : locationFrom(hit, { offset: signal.at?.offset ?? OFFSET.signals }),
            }), (s) => ops.update('signals', s)))} />
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
        return {
          id: `trackSections:${s.id}`, label: labelOf('trackSections', s, t), color: colors.get(s.id),
          sub: `${word(SECTION_KIND, s.kind, language)} · ${s.boundedBy.length} × · ${m} m`, end: bad.sections.has(s.id) ? <Badge tone="warn">!</Badge> : null,
        }
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

  const group = groups.find(g => g.key === listed)
  if (group) {
    return (
      <Panel title={group.title} back={{ label: t('back'), onClick: () => setListed(null) }}>
        <GroupedList label={group.title} groups={[group]} open selected={chosen} empty={t('edit_empty')} searchLabel={t('search_objects')} noneFound={t('search_none')}
          onSelect={(key) => {
            const at = key.indexOf(':')
            select({ collection: key.slice(0, at), id: key.slice(at + 1) })
          }} />
      </Panel>
    )
  }
  const option = (g, icon) => (
    <Option key={g.key} icon={icon} end={g.items.length} disabled={!g.items.length} onClick={() => setListed(g.key)}>{g.title}</Option>
  )
  const headed = [...new Set(groups.map(g => g.heading))]
  return (
    <Panel title={t('section_edit')} intro={t('section_edit_intro')}>
      {headed.map((heading, i) => (
        <div key={heading}>
          {i > 0 && <OptionsDivider />}
          <Options>
            <OptionsHeading>{heading}</OptionsHeading>
            {groups.filter(g => g.heading === heading).map(g => option(g, ICONS[g.key] ?? <BaliseIcon />))}
          </Options>
        </div>
      ))}
    </Panel>
  )
}
