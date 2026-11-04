import { useState } from 'react'
import { Button, Option, Options, Panel, SelectField, TextField } from '@rail-across/ui'
import { SIGNAL_CATALOGUE, protectionSystemOf, titleIn } from '@rail-across/core/signalling'
import { newBalise } from '../model/domain.js'
import { baliseKindOptions, baliseWord, effectOptions } from '../labels.js'
import { BaliseIcon } from '../icons.jsx'
import { Created } from './common.jsx'
import { OFFSET, locationFrom, pickingSignal, placing, signalOptions as signalChoices } from '../model/places.js'

/**
 * Creating balises and magnets: a page per train protection system the
 * catalogue declares (PZB, ETCS), which takes the kind, how it acts and the
 * signal it belongs to, then a click on the plan sets it — with the settings
 * its kind asks, to be told in its editor (Bearbeiten). The page stays, for
 * the next one.
 */
export default function BalisesPanel({ domain, net, ops, select, setMode, t, language }) {
  const [system, setSystem] = useState(null)
  const [made, setMade] = useState(null)
  const go = (next) => { setMade(null); setSystem(next) }
  const systems = Object.keys(SIGNAL_CATALOGUE.zugbeeinflussung.systeme)

  if (system) {
    return (
      <Panel title={t('create_balise', { system: protectionSystemOf(system).name })} intro={t('create_balise_intro')} back={{ label: t('back'), onClick: () => go(null) }}>
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
