import { useState } from 'react'
import { Button, Checkbox, Option, Options, OptionsDivider, OptionsHeading, Panel, SelectField, TextField } from '@rail-across/ui'
import { SIGNAL_CATALOGUE, titleIn } from '@rail-across/core/signalling'
import { labelOf, logicalId, newId, newLogical, newSignal } from '../model/domain.js'
import { FUNCTION, MOUNTING, VIRTUAL_FUNCTION, logicalSignalWord, optionsOf, signalSystemOptions, signalTypeOptions } from '../labels.js'
import { BoardIcon, LocationIcon, SignalIcon, VirtualIcon } from '../icons.jsx'
import { Created } from './common.jsx'
import { OFFSET, locationFrom, placing } from '../model/places.js'

/** The boards that can be set on their own: every aspect a plate may show. */
const BOARDS = SIGNAL_CATALOGUE.begriffe.filter(b => !b.nur_dv && b.rahmen?.includes('plate'))

/**
 * Creating signals, as the Layout-Tool's create panel: a menu, and a page per
 * kind of thing — a signal mast (the physical signal, bare: its logical
 * signals come after), a board of no system on its own post, a logical
 * signal (on a new mast set on the plan, or added to a mast there is), a
 * virtual signal. Each page takes what the thing is first, then a click on
 * the plan sets it; it stays, for the next one, and offers to edit the one
 * set (Bearbeiten).
 */
export default function SignalsPanel({ domain, net, ops, select, setMode, t, language }) {
  const [page, setPage] = useState('menu')
  const [made, setMade] = useState(null)
  const go = (next) => { setMade(null); setPage(next) }
  const back = { label: t('back'), onClick: () => go('menu') }
  const noNet = !net.tracks.length
  /** Set on the plan what `make` makes of the place clicked, and say so — what `madeOf` says was made, else the mast. */
  const place = (make, madeOf = (s) => ({ collection: 'signals', id: s.id, what: labelOf('signals', s, t) })) => setMode(placing('signals', make, (s) => {
    ops.add('signals', s)
    setMade(madeOf(s))
  }))
  const created = <Created what={made?.what} t={t} onEdit={() => select({ collection: made.collection, id: made.id })} />
  const props = { domain, place, noNet, created, t, language }

  if (page === 'mast') return <Panel title={t('create_mast')} intro={t('create_mast_intro')} back={back}><MastPage {...props} /></Panel>
  if (page === 'board') return <Panel title={t('create_board')} intro={t('create_board_intro')} back={back}><BoardPage {...props} /></Panel>
  if (page === 'virtual') return <Panel title={t('create_virtual')} intro={t('create_virtual_intro')} back={back}><VirtualPage {...props} /></Panel>
  if (page === 'logical') {
    return (
      <Panel title={t('create_logical')} intro={t('create_logical_intro')} back={back}>
        <LogicalPage {...props} onAdded={(mast, l) => {
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

/** The button that sets on the plan what the page describes. */
const PlaceButton = ({ onClick, disabled, t }) => (
  <div className="ui-row"><Button variant="primary" disabled={disabled} onClick={onClick}>{t('place_on_map')}</Button></div>
)

function MastPage({ place, noNet, created, t, language }) {
  const [name, setName] = useState('')
  const [mounting, setMounting] = useState('mast')
  const [both, setBoth] = useState(false)
  return <>
    <div className="ui-fields">
      <TextField compact label={t('name')} value={name} onChange={setName} />
      <SelectField compact label={t('mounting')} value={mounting} options={optionsOf(MOUNTING, language)} onChange={(v) => setMounting(v ?? 'mast')} />
      <div className="ui-wide"><Checkbox label={t('both_directions')} checked={both} onChange={setBoth} /></div>
    </div>
    <PlaceButton t={t} disabled={noNet} onClick={() => place((hit) => ({
      ...newSignal({ at: locationFrom(hit, { offset: OFFSET.signals }), name: name || undefined, mounting, bare: true }),
      ...(both ? { bothDirections: true } : {}),
    }))} />
    {created}
  </>
}

function BoardPage({ place, noNet, created, t, language }) {
  const [aspect, setAspect] = useState('Ne_1')
  const [name, setName] = useState('')
  return <>
    <div className="ui-fields">
      <SelectField compact wide label={t('board_shows')} value={aspect} onChange={(v) => setAspect(v ?? 'Ne_1')}
        options={BOARDS.map(b => ({ value: b.id, label: b.title ? `${b.name} — ${titleIn(b, language)}` : b.name }))} />
      <TextField compact wide label={t('name')} value={name} onChange={setName} />
    </div>
    <p className="ui-muted">{t('create_board_hint')}</p>
    <PlaceButton t={t} disabled={noNet} onClick={() => place((hit) => ({
      id: newId(), name: name || undefined, kind: 'real', function: 'standaloneIndicator', mounting: 'mast',
      at: locationFrom(hit, { offset: OFFSET.signals }), frames: [{ kind: 'plate', aspects: [{ aspect }] }],
    }))} />
    {created}
  </>
}

function VirtualPage({ place, noNet, created, t, language }) {
  const [fn, setFn] = useState('trainTargetLine')
  const [name, setName] = useState('')
  return <>
    <div className="ui-fields">
      <SelectField compact wide label={t('virtual_function')} value={fn} options={optionsOf(VIRTUAL_FUNCTION, language)} onChange={(v) => setFn(v ?? 'trainTargetLine')} />
      <TextField compact wide label={t('name')} value={name} onChange={setName} />
    </div>
    <PlaceButton t={t} disabled={noNet} onClick={() => place((hit) => ({
      id: newId(), name: name || undefined, kind: 'virtual', virtualFunction: fn, at: locationFrom(hit, { offset: OFFSET.signals }),
    }))} />
    {created}
  </>
}

/**
 * A logical signal: its system, kind and function; then where — on a new
 * mast set on the plan, or on a mast there is that shows nothing of that
 * system yet.
 */
function LogicalPage({ domain, place, noNet, created, onAdded, t, language }) {
  const [system, setSystem] = useState('ks')
  const firstType = (sys) => Object.keys(SIGNAL_CATALOGUE.systeme[sys]?.signalarten ?? {})[0]
  const [type, setType] = useState(firstType('ks'))
  const [fn, setFn] = useState('entry')
  const [where, setWhere] = useState('new')
  const masts = domain.collections.signals
    .filter(s => s.kind === 'real' && !(s.logicalSignals ?? []).some(l => l.system === system))
    .map(s => ({ value: s.id, label: labelOf('signals', s, t) }))
    .sort((a, b) => String(a.label).localeCompare(String(b.label), 'de', { numeric: true }))
  const [mastId, setMastId] = useState(null)
  const mast = masts.some(m => m.value === mastId) ? mastId : masts[0]?.value ?? null
  const make = () => newLogical({ system, type, fn })
  return <>
    <div className="ui-fields">
      <SelectField compact wide label={t('system')} value={system} options={signalSystemOptions(language)}
        onChange={(v) => { setSystem(v); setType(firstType(v)) }} />
      <SelectField compact wide label={t('type')} value={type ?? null} options={signalTypeOptions(system, language, t)} onChange={(v) => setType(v ?? undefined)} />
      <SelectField compact wide label={t('function')} value={fn} options={optionsOf(FUNCTION, language)} onChange={(v) => setFn(v ?? 'entry')} />
      <SelectField compact wide label={t('create_where')} value={where} onChange={setWhere}
        options={[{ value: 'new', label: t('create_where_new') }, { value: 'mast', label: t('create_where_mast'), disabled: !masts.length }]} />
      {where === 'mast' && (
        <SelectField compact wide label={t('location_of_signal')} value={mast} options={masts} onChange={setMastId} />
      )}
    </div>
    {where === 'new' ? (
      <PlaceButton t={t} disabled={noNet} onClick={() => place(
        (hit) => ({ ...newSignal({ at: locationFrom(hit, { offset: OFFSET.signals }), bare: true }), logicalSignals: [make()] }),
        (s) => ({ collection: 'logicalSignals', id: logicalId(s.id, system), what: `${labelOf('signals', s, t)} · ${logicalSignalWord(s.logicalSignals[0], language)}` }),
      )} />
    ) : (
      <div className="ui-row">
        <Button variant="primary" disabled={!mast} onClick={() => onAdded(domain.collections.signals.find(s => s.id === mast), make())}>{t('create_add')}</Button>
      </div>
    )}
    {created}
  </>
}
