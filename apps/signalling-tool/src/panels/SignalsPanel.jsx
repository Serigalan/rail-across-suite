import { useMemo, useState } from 'react'
import { Button, Checkbox, NumberField, Option, Options, OptionsDivider, OptionsHeading, Section, SelectField, TextField } from '@rail-across/ui'
import { Panel } from './draft.jsx'
import { SIGNAL_CATALOGUE, titleIn } from '@rail-across/core/signalling'
import { labelOf, logicalId, newId, newLogical, newSignal } from '../model/domain.js'
import { VIRTUAL_FUNCTION, functionOfChoice, functionOptions, logicalSignalWord, mountingOptions, optionsOf, signalSystemOptions, signalTypeOptions, typeForChoice } from '../labels.js'
import { BoardIcon, LocationIcon, SignalIcon, VirtualIcon } from '../icons.jsx'
import { Created } from './common.jsx'
import { useFields, useSetting } from './useSetting.js'
import { BOARD_ASPECTS, BRACKET_OFFSET, FOUNDATION_OFFSET, OFFSET, ON_FOUNDATION, bracketScreen, locationFrom, mountedFrom, signalOptions } from '../model/places.js'
import { SPEED_SIGNAL, isSpeedSignal, speedSignalSpeeds, withSpeedSignalSpeed } from '../model/routeSpeeds.js'
import { SpeedReach } from './speedReach.jsx'

/** The boards that can be set on their own: those needed (BOARD_ASPECTS). */
const BOARDS = BOARD_ASPECTS.map(id => SIGNAL_CATALOGUE.begriffe.find(b => b.id === id)).filter(Boolean)

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
export default function SignalsPanel({ domain, net, ops, select, mode, setMode, pending, setSketch, showOnPlan, t, language }) {
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
  const props = { domain, net, offer, remake, noNet, created, made, setSketch, showOnPlan, t, language }

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

/**
 * Where a gantry's or a bracket's screen and foundation stand until they are
 * told: a gantry's screen over the track, a bracket's on the other side than
 * its mast (`bracket` its offset), the foundation on the right.
 */
const MOUNTED = { screen: { side: null, offset: 0 }, foundation: { side: 'right', offset: FOUNDATION_OFFSET }, bracket: BRACKET_OFFSET }

/** A new mast's fields as they start: no name, no area, on a mast, one direction. */
const MAST = { name: '', areaNumber: '', mounting: 'mast', mounted: MOUNTED, both: false }

/** Where a signal on that mounting is set, from where the plan was hit: its screen and its foundation where asked, else the plain offset. */
const whereOf = (hit, mounting, mounted) => (mounting === 'bracket'
  ? mountedFrom(hit, { ...mounted, screen: bracketScreen(mounted.foundation, mounted.bracket) })
  : ON_FOUNDATION.has(mounting) ? mountedFrom(hit, mounted) : { at: locationFrom(hit, { offset: OFFSET.signals }) })

/** The bare mast the fields describe, set where the plan was hit. */
const mastAt = (hit, { name, areaNumber, mounting, mounted, both }) => ({
  ...newSignal({ ...whereOf(hit, mounting, mounted), name: name || undefined, mounting, bare: true }),
  ...(areaNumber ? { areaNumber } : {}),
  ...(both ? { bothDirections: true } : {}),
})

/**
 * Where a gantry's or a bracket's screen stands and where its foundation
 * does, each side seen in the direction the signal acts: a bracket's screen
 * on the other side than its mast, at an offset from the axis; a gantry's
 * over the track or on a side, its second mast across from the first.
 */
function MountedFields({ value, mounting, onChange, t }) {
  const set = (part, patch) => onChange({ ...value, [part]: { ...value[part], ...patch } })
  const sides = [{ value: 'left', label: t('left') }, { value: 'right', label: t('right') }]
  return <>
    <Section title={t('foundation_location')}>
      <div className="ui-fields">
        <SelectField compact label={t('side_along')} value={value.foundation.side} options={sides} onChange={(v) => set('foundation', { side: v ?? 'right' })} />
        <NumberField compact label={t('offset')} value={value.foundation.offset} min={0} onChange={(v) => set('foundation', { offset: v ?? FOUNDATION_OFFSET })} />
      </div>
    </Section>
    <Section title={t('screen_location')}>
      {mounting === 'bracket'
        ? <div className="ui-fields">
          <NumberField compact label={t('bracket_offset')} value={value.bracket} min={0} onChange={(v) => onChange({ ...value, bracket: v ?? BRACKET_OFFSET })} />
        </div>
        : <div className="ui-fields">
          <SelectField compact label={t('side_along')} value={value.screen.side} empty={t('over_track')} options={sides} onChange={(v) => set('screen', { side: v ?? null })} />
          <NumberField compact label={t('offset')} value={value.screen.side ? value.screen.offset : 0} min={0} disabled={!value.screen.side}
            onChange={(v) => set('screen', { offset: v ?? 0 })} />
        </div>}
    </Section>
    <p className="ui-muted">{t(mounting === 'bracket' ? 'bracket_hint' : 'gantry_hint')}</p>
  </>
}

/** A new mast's fields: its area's number and name (21-B), what it stands on, both directions; on a gantry or a bracket where its screen and foundation stand. */
function MastFields({ value, set, t, language }) {
  return <>
    <div className="ui-fields">
      <TextField compact label={t('area')} value={value.areaNumber} onChange={(areaNumber) => set({ areaNumber: areaNumber.trim() })} />
      <TextField compact label={t('name')} value={value.name} onChange={(name) => set({ name })} />
      <SelectField compact wide label={t('mounting')} value={value.mounting} options={mountingOptions(language, value.mounting)} onChange={(v) => set({ mounting: v ?? 'mast' })} />
      <div className="ui-wide"><Checkbox label={t('both_directions')} checked={value.both} onChange={(both) => set({ both })} /></div>
    </div>
    {ON_FOUNDATION.has(value.mounting) && <MountedFields value={value.mounted} mounting={value.mounting} onChange={(mounted) => set({ mounted })} t={t} />}
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

/**
 * A board of no system on its own post, one of those needed: one with a
 * figure with it — a speed signal Lf 7 by default showing, in tens, the
 * lowest speed the elements allow from where it is set to the next speed
 * signal, Zs 3 or Lf 7, ahead, as it is drawn under the mouse already; once
 * set, how far it holds and the element that sets its speed.
 */
function BoardPage({ domain, net, offer, remake, noNet, created, made, setSketch, showOnPlan, t, language }) {
  const symbolsOf = (aspect) => BOARDS.find(b => b.id === aspect)?.symbole ?? null
  // The mouse asks for it as it moves.
  const elementSpeedOf = useMemo(() => speedSignalSpeeds(net, domain.collections.signals), [net, domain.collections.signals])
  const makeOf = ({ aspect, symbol, auto, name }) => (hit, id) => {
    const s = {
      id: id ?? newId(), name: name || undefined, kind: 'real', function: 'standaloneIndicator', mounting: 'mast',
      at: locationFrom(hit, { offset: OFFSET.signals }), frames: [{ kind: 'plate', aspects: [{ aspect, ...(symbolsOf(aspect) && symbol ? { symbol } : {}) }] }],
    }
    return aspect === SPEED_SIGNAL && auto ? withSpeedSignalSpeed(s, elementSpeedOf(s)) : s
  }
  const [f, set] = useFields({ aspect: 'Ne_1', symbol: null, auto: true, name: '' }, remake, makeOf)
  offer(makeOf(f))
  const symbols = symbolsOf(f.aspect)
  const fromElements = f.aspect === SPEED_SIGNAL && f.auto
  // The Lf 7 set last, while it is a preview or once committed.
  const setSpeed = domain.collections.signals.find(s => s.id === (made?.objectId ?? made?.id) && isSpeedSignal(s)) ?? null
  // A board that shows a speed shows it in tens: Zs 3 (6) is 60 km/h.
  const symbolLabel = (x) => (BOARDS.find(b => b.id === f.aspect)?.symbol_bedeutung === 'speed' ? `${x} — ${Number(x.replace(',', '.')) * 10} km/h` : x)
  return <>
    <div className="ui-fields">
      <SelectField compact wide label={t('board_shows')} value={f.aspect}
        onChange={(v) => { const aspect = v ?? 'Ne_1'; set({ aspect, symbol: symbolsOf(aspect)?.[0] ?? null }) }}
        options={BOARDS.map(b => ({ value: b.id, label: b.title ? `${b.name} — ${titleIn(b, language)}` : b.name }))} />
      {f.aspect === SPEED_SIGNAL && <div className="ui-check-cell ui-wide"><Checkbox label={t('speed_from_elements')} checked={f.auto} onChange={(auto) => set({ auto })} /></div>}
      {symbols && !fromElements && <SelectField compact wide label={t('board_symbol')} value={f.symbol} options={symbols.map(x => ({ value: x, label: symbolLabel(x) }))} onChange={(symbol) => set({ symbol })} />}
      <TextField compact wide label={t('name')} value={f.name} onChange={(name) => set({ name })} />
    </div>
    {f.aspect === SPEED_SIGNAL && <p className="ui-muted">{t('create_speed_signal_hint')}</p>}
    {setSpeed && <SpeedReach signal={setSpeed} domain={domain} net={net} setSketch={setSketch} showOnPlan={showOnPlan} t={t} />}
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
  const makeOf = ({ system, type, fn, mast }) => (hit) => ({ ...mastAt(hit, mast), logicalSignals: [newLogical({ system, type, fn: functionOfChoice(fn) })] })
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
      <SelectField compact wide label={t('function')} value={f.fn} options={functionOptions(language)}
        onChange={(v) => { const fn = v ?? 'entry'; set({ fn, type: typeForChoice(f.system, fn, f.type) }) }} />
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
