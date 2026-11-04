import { useState } from 'react'
import { Button, Checkbox, NumberField, Option, Options, OptionsDivider, OptionsHeading, Section, SelectField, TextField } from '@rail-across/ui'
import { Panel } from './draft.jsx'
import { SIGNAL_CATALOGUE, titleIn } from '@rail-across/core/signalling'
import { labelOf, logicalId, newId, newLogical, newSignal } from '../model/domain.js'
import { FUNCTION, MOUNTING, VIRTUAL_FUNCTION, logicalSignalWord, optionsOf, signalSystemOptions, signalTypeOptions } from '../labels.js'
import { BoardIcon, LocationIcon, SignalIcon, VirtualIcon } from '../icons.jsx'
import { Created } from './common.jsx'
import { useFields, useSetting } from './useSetting.js'
import { FOUNDATION_OFFSET, OFFSET, ON_FOUNDATION, locationFrom, mountedFrom, signalOptions } from '../model/places.js'

/** The boards that can be set on their own: every aspect a plate may show. */
const BOARDS = SIGNAL_CATALOGUE.begriffe.filter(b => !b.nur_dv && b.rahmen?.includes('plate'))

/**
 * Creating signals, as the Layout-Tool's create panel: a menu, and a page per
 * kind of thing — a signal mast (the physical signal, bare: its logical
 * signals come after), a board of no system on its own post, a logical
 * signal (on a new mast set on the plan, with the mast's own fields, or
 * added to a mast there is), a virtual signal. Each page takes what the
 * thing is; while it is open the plan sets it — the mouse snaps to a track,
 * the signal is drawn there as the fields say, and a click sets it as a
 * preview, which the page's fields go on changing and a further click moves,
 * till it is committed at the panel's foot; then a click sets the next one.
 * The page offers to edit the one set (Bearbeiten).
 */
export default function SignalsPanel({ domain, net, ops, select, mode, setMode, pending, t, language }) {
  const [page, setPage] = useState('menu')
  const [where, setWhere] = useState('new')
  const noNet = !net.tracks.length
  const { made, setMade, offer: offerMade, remake } = useSetting({
    collection: 'signals', active: page !== 'menu' && !(page === 'logical' && where === 'mast') && !noNet,
    mode, setMode, ops, pending, hints: ['place_signal', 'place_signal_move'], t,
  })
  const offer = (make, madeOf = mastMade(t)) => offerMade(make, madeOf)
  const go = (next) => { setMade(null); setWhere('new'); setPage(next) }
  const back = { label: t('back'), onClick: () => go('menu') }
  const created = <Created what={made?.what} t={t} onEdit={() => select({ collection: made.collection, id: made.id })} />
  const props = { domain, net, offer, remake, noNet, created, t, language }

  if (page === 'mast') return <Panel title={t('create_mast')} intro={t('create_mast_intro')} back={back}><MastPage {...props} /></Panel>
  if (page === 'board') return <Panel title={t('create_board')} intro={t('create_board_intro')} back={back}><BoardPage {...props} /></Panel>
  if (page === 'virtual') return <Panel title={t('create_virtual')} intro={t('create_virtual_intro')} back={back}><VirtualPage {...props} /></Panel>
  if (page === 'logical') {
    return (
      <Panel title={t('create_logical')} intro={t('create_logical_intro')} back={back}>
        <LogicalPage {...props} where={where} setWhere={setWhere} onAdded={(mast, l) => {
          ops.update('signals', { ...mast, logicalSignals: [...(mast.logicalSignals ?? []), l] })
          setMade({ collection: 'logicalSignals', id: logicalId(mast.id, l.system), what: `${labelOf('signals', mast, t)} · ${logicalSignalWord(l, language)}` })
        }} />
      </Panel>
    )
  }

  return (
    <Panel title={t('section_signals')} intro={t('section_signals_intro')}>
      <Options>
        <OptionsHeading>{t('signals_physical')}</OptionsHeading>
        <Option icon={<LocationIcon />} onClick={() => go('mast')}>{t('create_mast')}</Option>
        <Option icon={<BoardIcon />} onClick={() => go('board')}>{t('create_board')}</Option>
      </Options>
      <OptionsDivider />
      <Options>
        <OptionsHeading>{t('logical_signals')}</OptionsHeading>
        <Option icon={<SignalIcon />} onClick={() => go('logical')}>{t('create_logical')}</Option>
      </Options>
      <OptionsDivider />
      <Options>
        <OptionsHeading>{t('signals_virtual')}</OptionsHeading>
        <Option icon={<VirtualIcon />} onClick={() => go('virtual')}>{t('create_virtual')}</Option>
      </Options>
    </Panel>
  )
}

/** What the page says where the net has no track to set on. */
const NoNet = ({ noNet, t }) => (noNet ? <p className="ui-muted">{t('empty_net')}</p> : null)

/** What a mast set is called, and the way to its editor. */
const mastMade = (t) => (s) => ({ collection: 'signals', id: s.id, what: labelOf('signals', s, t) })

/** Where a gantry's or a bracket's screen and foundation stand until they are told: the screen over the track, the foundation on the right. */
const MOUNTED = { screen: { side: null, offset: 0 }, foundation: { side: 'right', offset: FOUNDATION_OFFSET } }

/** A new mast's fields as they start: no name, no area, on a mast, one direction. */
const MAST = { name: '', areaNumber: '', mounting: 'mast', mounted: MOUNTED, both: false }

/** Where a signal on that mounting is set, from where the plan was hit: its screen and its foundation where asked, else the plain offset. */
const whereOf = (hit, mounting, mounted) => (ON_FOUNDATION.has(mounting) ? mountedFrom(hit, mounted) : { at: locationFrom(hit, { offset: OFFSET.signals }) })

/** The bare mast the fields describe, set where the plan was hit. */
const mastAt = (hit, { name, areaNumber, mounting, mounted, both }) => ({
  ...newSignal({ ...whereOf(hit, mounting, mounted), name: name || undefined, mounting, bare: true }),
  ...(areaNumber ? { areaNumber } : {}),
  ...(both ? { bothDirections: true } : {}),
})

/**
 * Where a gantry's or a bracket's screen stands — over the track or on a
 * side, at an offset from its axis — and where its foundation does, each
 * side seen in the direction the signal acts.
 */
function MountedFields({ value, onChange, t }) {
  const set = (part, patch) => onChange({ ...value, [part]: { ...value[part], ...patch } })
  const sides = [{ value: 'left', label: t('left') }, { value: 'right', label: t('right') }]
  return <>
    <Section title={t('screen_location')}>
      <div className="ui-fields">
        <SelectField compact label={t('side_along')} value={value.screen.side} empty={t('over_track')} options={sides} onChange={(v) => set('screen', { side: v ?? null })} />
        <NumberField compact label={t('offset')} value={value.screen.side ? value.screen.offset : 0} min={0} disabled={!value.screen.side}
          onChange={(v) => set('screen', { offset: v ?? 0 })} />
      </div>
    </Section>
    <Section title={t('foundation_location')}>
      <div className="ui-fields">
        <SelectField compact label={t('side_along')} value={value.foundation.side} options={sides} onChange={(v) => set('foundation', { side: v ?? 'right' })} />
        <NumberField compact label={t('offset')} value={value.foundation.offset} min={0} onChange={(v) => set('foundation', { offset: v ?? FOUNDATION_OFFSET })} />
      </div>
    </Section>
    <p className="ui-muted">{t('mounted_hint')}</p>
  </>
}

/** A new mast's fields: its area's number and name (21-B), what it stands on, both directions; on a gantry or a bracket where its screen and foundation stand. */
function MastFields({ value, set, t, language }) {
  return <>
    <div className="ui-fields">
      <TextField compact label={t('area')} value={value.areaNumber} onChange={(areaNumber) => set({ areaNumber: areaNumber.trim() })} />
      <TextField compact label={t('name')} value={value.name} onChange={(name) => set({ name })} />
      <SelectField compact wide label={t('mounting')} value={value.mounting} options={optionsOf(MOUNTING, language)} onChange={(v) => set({ mounting: v ?? 'mast' })} />
      <div className="ui-wide"><Checkbox label={t('both_directions')} checked={value.both} onChange={(both) => set({ both })} /></div>
    </div>
    {ON_FOUNDATION.has(value.mounting) && <MountedFields value={value.mounted} onChange={(mounted) => set({ mounted })} t={t} />}
  </>
}

function MastPage({ offer, remake, noNet, created, t, language }) {
  const makeOf = (f) => (hit) => mastAt(hit, f)
  const [mast, setMast] = useFields(MAST, remake, makeOf)
  offer(makeOf(mast))
  return <>
    <MastFields value={mast} set={setMast} t={t} language={language} />
    <NoNet noNet={noNet} t={t} />
    {created}
  </>
}

function BoardPage({ offer, remake, noNet, created, t, language }) {
  const makeOf = ({ aspect, name }) => (hit) => ({
    id: newId(), name: name || undefined, kind: 'real', function: 'standaloneIndicator', mounting: 'mast',
    at: locationFrom(hit, { offset: OFFSET.signals }), frames: [{ kind: 'plate', aspects: [{ aspect }] }],
  })
  const [f, set] = useFields({ aspect: 'Ne_1', name: '' }, remake, makeOf)
  offer(makeOf(f))
  return <>
    <div className="ui-fields">
      <SelectField compact wide label={t('board_shows')} value={f.aspect} onChange={(v) => set({ aspect: v ?? 'Ne_1' })}
        options={BOARDS.map(b => ({ value: b.id, label: b.title ? `${b.name} — ${titleIn(b, language)}` : b.name }))} />
      <TextField compact wide label={t('name')} value={f.name} onChange={(name) => set({ name })} />
    </div>
    <p className="ui-muted">{t('create_board_hint')}</p>
    <NoNet noNet={noNet} t={t} />
    {created}
  </>
}

function VirtualPage({ offer, remake, noNet, created, t, language }) {
  const makeOf = ({ fn, name }) => (hit) => ({
    id: newId(), name: name || undefined, kind: 'virtual', virtualFunction: fn, at: locationFrom(hit, { offset: OFFSET.signals }),
  })
  const [f, set] = useFields({ fn: 'trainTargetLine', name: '' }, remake, makeOf)
  offer(makeOf(f))
  return <>
    <div className="ui-fields">
      <SelectField compact wide label={t('virtual_function')} value={f.fn} options={optionsOf(VIRTUAL_FUNCTION, language)} onChange={(v) => set({ fn: v ?? 'trainTargetLine' })} />
      <TextField compact wide label={t('name')} value={f.name} onChange={(name) => set({ name })} />
    </div>
    <NoNet noNet={noNet} t={t} />
    {created}
  </>
}

/** A logical signal set on a new mast is called by both, and opens in its own editor. */
const logicalMade = (t, language) => (s) => ({
  collection: 'logicalSignals', id: logicalId(s.id, s.logicalSignals[0].system),
  what: `${labelOf('signals', s, t)} · ${logicalSignalWord(s.logicalSignals[0], language)}`,
})

/**
 * A logical signal: its system, kind and function; then where — on a new
 * mast set on the plan, whose fields the page then asks as the mast's page
 * does, its area's number too, or on a mast there is that shows nothing of
 * that system yet.
 */
function LogicalPage({ domain, net, offer, remake, noNet, created, onAdded, where, setWhere, t, language }) {
  const firstType = (sys) => Object.keys(SIGNAL_CATALOGUE.systeme[sys]?.signalarten ?? {})[0]
  const makeOf = ({ system, type, fn, mast }) => (hit) => ({ ...mastAt(hit, mast), logicalSignals: [newLogical({ system, type, fn })] })
  const [f, set] = useFields({ system: 'ks', type: firstType('ks'), fn: 'entry', mast: MAST }, remake, makeOf)
  offer(makeOf(f), logicalMade(t, language))
  const masts = signalOptions(net, domain.collections.signals
    .filter(s => s.kind === 'real' && !(s.logicalSignals ?? []).some(l => l.system === f.system)), t)
  const [mastId, setMastId] = useState(null)
  const mast = masts.some(m => m.value === mastId) ? mastId : masts[0]?.value ?? null
  return <>
    <div className="ui-fields">
      <SelectField compact wide label={t('system')} value={f.system} options={signalSystemOptions(language)}
        onChange={(v) => set({ system: v, type: firstType(v) })} />
      <SelectField compact wide label={t('type')} value={f.type ?? null} options={signalTypeOptions(f.system, language, t)} onChange={(v) => set({ type: v ?? undefined })} />
      <SelectField compact wide label={t('function')} value={f.fn} options={optionsOf(FUNCTION, language)} onChange={(v) => set({ fn: v ?? 'entry' })} />
      <SelectField compact wide label={t('create_where')} value={where} onChange={setWhere}
        options={[{ value: 'new', label: t('create_where_new') }, { value: 'mast', label: t('create_where_mast'), disabled: !masts.length }]} />
      {where === 'mast' && (
        <SelectField compact wide label={t('location_of_signal')} value={mast} options={masts} onChange={setMastId} />
      )}
    </div>
    {where === 'new' && (
      <Section title={t('new_mast')}>
        <MastFields value={f.mast} set={(patch) => set({ mast: { ...f.mast, ...patch } })} t={t} language={language} />
      </Section>
    )}
    {where === 'new' ? <NoNet noNet={noNet} t={t} /> : (
      <div className="ui-row">
        <Button variant="primary" disabled={!mast} onClick={() => onAdded(domain.collections.signals.find(s => s.id === mast), newLogical(f))}>{t('create_add')}</Button>
      </div>
    )}
    {created}
  </>
}
