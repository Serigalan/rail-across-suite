import { useState } from 'react'
import { Button, Option, Options, OptionsDivider, SelectField, TextField } from '@rail-across/ui'
import { Panel } from './draft.jsx'
import { SIGNAL_CATALOGUE, protectionSystemOf, titleIn } from '@rail-across/core/signalling'
import { newBalise } from '../model/domain.js'
import { baliseKindOptions, baliseWord, effectOptions } from '../labels.js'
import { BaliseIcon, SignalIcon } from '../icons.jsx'
import { CheckList, Created } from './common.jsx'
import { magnetsDue, signalMagnet } from '../model/magnets.js'
import { OFFSET, locationFrom, pickingSignal, placing, signalOptions as signalChoices } from '../model/places.js'

/**
 * Creating balises and magnets: a page per train protection system the
 * catalogue declares (PZB, ETCS), which takes the kind, how it acts and the
 * signal it belongs to, then a click on the plan sets it — with the settings
 * its kind asks, to be told in its editor (Bearbeiten). The page stays, for
 * the next one. Signals can be equipped with magnets all at once, each its
 * own at its place (magnets.js).
 */
export default function BalisesPanel({ domain, net, ops, select, setMode, t, language }) {
  const [system, setSystem] = useState(null)
  const [made, setMade] = useState(null)
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
        <BalisePage key={system} system={system} domain={domain} net={net} setMode={setMode} t={t} language={language}
          onPlace={(make) => setMode(placing('balises', make, (b) => {
            ops.add('balises', b)
            setMade({ id: b.id, what: [b.name, baliseWord(b, language, t)].filter(Boolean).join(' · ') })
          }))} />
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

function BalisePage({ system, domain, net, setMode, onPlace, t, language }) {
  const kinds = baliseKindOptions(system, language)
  const [kind, setKind] = useState(kinds[0]?.value)
  const [effect, setEffect] = useState(system === 'pzb' ? 'switchedBySignal' : 'permanent')
  const [signalId, setSignalId] = useState(null)
  const [name, setName] = useState('')
  const real = domain.collections.signals.filter(s => s.kind === 'real')
  const signalOptions = signalChoices(net, real, t)
  return <>
    <div className="ui-fields">
      <SelectField compact wide label={t('kind')} value={kind} options={kinds} onChange={setKind} />
      <SelectField compact wide label={t('effect')} value={effect} options={effectOptions(language)} onChange={setEffect} />
      <SelectField compact wide label={t('belongs_to')} value={signalId} empty={t('none')} options={signalOptions} onChange={setSignalId} />
      <div className="ui-wide ui-row">
        <Button size="small" disabled={!real.length}
          onClick={() => setMode(pickingSignal(real, t, (id) => { setSignalId(id); setMode(null) }))}>{t('pick_signal_on_plan')}</Button>
      </div>
      <TextField compact wide label={t('name')} value={name} onChange={setName} />
    </div>
    <div className="ui-row">
      <Button variant="primary" disabled={!net.tracks.length}
        onClick={() => onPlace((hit) => ({
          ...newBalise({ at: locationFrom(hit, { offset: OFFSET.balises }), system, kind, effect, signalId: signalId ?? undefined }),
          ...(name ? { name } : {}),
        }))}>
        {t('place_on_map')}
      </Button>
    </div>
  </>
}

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
