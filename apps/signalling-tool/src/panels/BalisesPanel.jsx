import { useState } from 'react'
import { Button, Option, Options, OptionsDivider, SelectField, TextField } from '@rail-across/ui'
import { Panel } from './draft.jsx'
import { SIGNAL_CATALOGUE, protectionSystemOf, titleIn } from '@rail-across/core/signalling'
import { newBalise } from '../model/domain.js'
import { baliseKindOptions, baliseWord, effectOptions } from '../labels.js'
import { BaliseIcon, SignalIcon } from '../icons.jsx'
import { CheckList, Created } from './common.jsx'
import { magnetsDue, signalMagnet } from '../model/magnets.js'
import { OFFSET, locationFrom, pickingSignal, signalOptions as signalChoices } from '../model/places.js'
import { useFields, useSetting } from './useSetting.js'

/**
 * Creating balises and magnets: a page per train protection system the
 * catalogue declares (PZB, ETCS), which takes the kind, how it acts and the
 * signal it belongs to; while it is open a click on the plan sets it, as a
 * preview till it is committed (useSetting) — with the settings its kind
 * asks, to be told in its editor (Bearbeiten). Signals can be equipped with magnets all at once, each its
 * own at its place (magnets.js).
 */
export default function BalisesPanel({ domain, net, ops, select, mode, setMode, pending, t, language }) {
  const [system, setSystem] = useState(null)
  const { made, setMade, offer, remake } = useSetting({
    collection: 'balises', active: !!system && system !== 'equip' && net.tracks.length > 0,
    mode, setMode, ops, pending, hints: ['place_balise', 'place_balise_move'], t,
  })
  const go = (next) => { setMade(null); setSystem(next) }
  const systems = Object.keys(SIGNAL_CATALOGUE.zugbeeinflussung.systeme)
  const back = { label: t('back'), onClick: () => go(null) }

  if (system === 'equip') {
    return (
      <Panel title={t('equip_magnets')} intro={t('equip_magnets_intro')} back={back}>
        <EquipPage domain={domain} net={net} ops={ops} t={t} />
      </Panel>
    )
  }

  if (system) {
    return (
      <Panel title={t('create_balise', { system: protectionSystemOf(system).name })} intro={t('create_balise_intro')} back={back}>
        <BalisePage key={system} system={system} domain={domain} net={net} setMode={setMode} offer={offer} remake={remake} t={t} language={language} />
        <Created what={made?.what} t={t} onEdit={() => select({ collection: 'balises', id: made.id })} />
      </Panel>
    )
  }

  return (
    <Panel title={t('section_balises')} intro={t('section_balises_intro')}>
      <Options>
        {systems.map(s => (
          <Option key={s} icon={<BaliseIcon />} onClick={() => go(s)}>{t('create_balise', { system: titleIn(protectionSystemOf(s), language) })}</Option>
        ))}
      </Options>
      <OptionsDivider />
      <Options>
        <Option icon={<SignalIcon />} onClick={() => go('equip')}>{t('equip_magnets')}</Option>
      </Options>
    </Panel>
  )
}

/** The kind of a balise or magnet, how it acts, the signal it belongs to and its name; the signal picked in the list or on the plan. */
function BalisePage({ system, domain, net, setMode, offer, remake, t, language }) {
  const kinds = baliseKindOptions(system, language)
  const makeOf = ({ kind, effect, signalId, name }) => (hit) => ({
    ...newBalise({ at: locationFrom(hit, { offset: OFFSET.balises }), system, kind, effect, signalId: signalId ?? undefined }),
    ...(name ? { name } : {}),
  })
  const [f, set] = useFields({ kind: kinds[0]?.value, effect: system === 'pzb' ? 'switchedBySignal' : 'permanent', signalId: null, name: '' }, remake, makeOf)
  offer(makeOf(f), baliseMade(t, language))
  const real = domain.collections.signals.filter(s => s.kind === 'real')
  const signalOptions = signalChoices(net, real, t)
  return <>
    <div className="ui-fields">
      <SelectField compact wide label={t('kind')} value={f.kind} options={kinds} onChange={(kind) => set({ kind })} />
      <SelectField compact wide label={t('effect')} value={f.effect} options={effectOptions(language)} onChange={(effect) => set({ effect })} />
      <SelectField compact wide label={t('belongs_to')} value={f.signalId} empty={t('none')} options={signalOptions} onChange={(signalId) => set({ signalId })} />
      <div className="ui-wide ui-row">
        <Button size="small" disabled={!real.length}
          onClick={() => setMode(pickingSignal(real, t, (id) => { set({ signalId: id }); setMode(null) }))}>{t('pick_signal_on_plan')}</Button>
      </div>
      <TextField compact wide label={t('name')} value={f.name} onChange={(name) => set({ name })} />
    </div>
    {!net.tracks.length && <p className="ui-muted">{t('empty_net')}</p>}
  </>
}

/** What a balise or magnet set is called, and the way to its editor. */
const baliseMade = (t, language) => (b) => ({ collection: 'balises', id: b.id, what: [b.name, baliseWord(b, language, t)].filter(Boolean).join(' · ') })

/**
 * Equipping the signals with magnets: those that ask for one and have none,
 * each with the frequency it gets, all ticked; the ones ticked get theirs at
 * once — a draft, drawn on the plan till it is confirmed.
 */
function EquipPage({ domain, net, ops, t }) {
  const { due, equipped } = magnetsDue(domain)
  const [skipped, setSkipped] = useState(() => new Set())
  const [done, setDone] = useState(0)
  const frequencyOf = new Map(due.map(d => [d.signal.id, d.frequency]))
  const options = signalChoices(net, due.map(d => d.signal), t)
    .map(o => ({ ...o, label: `${o.label} — ${frequencyOf.get(o.value).replace('+', '/')} Hz` }))
  const chosen = due.filter(d => !skipped.has(d.signal.id))
  const equip = () => {
    const magnets = chosen.map(d => signalMagnet(d.signal, d.frequency))
    ops.replaceAll({ fields: domain.fields, collections: { balises: [...domain.collections.balises, ...magnets] } })
    setDone(magnets.length)
  }
  return <>
    {done > 0 && <p className="tool-created" role="status">{t('equip_done', { n: done })}</p>}
    {equipped > 0 && <p className="ui-muted">{t('equip_have', { n: equipped })}</p>}
    {due.length ? <>
      <CheckList options={options} value={chosen.map(d => d.signal.id)}
        onChange={(ids) => setSkipped(new Set(due.map(d => d.signal.id).filter(id => !ids.includes(id))))} />
      <div className="ui-row">
        <Button variant="primary" disabled={!chosen.length} onClick={equip}>{t('equip_set', { n: chosen.length })}</Button>
      </div>
    </> : <p className="ui-muted">{t('equip_none_due')}</p>}
  </>
}
