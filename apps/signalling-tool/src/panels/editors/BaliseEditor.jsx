import { Checkbox, NumberField, Section, SelectField, TextField } from '@rail-across/ui'
import { baliseKindOf, protectionSystemOf, titleIn } from '@rail-across/core/signalling'
import { baliseSettings, labelOf } from '../../model/domain.js'
import { baliseKindOptions, baliseWord, effectOptions, protectionSystemOptions } from '../../labels.js'
import { EditorHead, LocationFields, ObjectFindings } from '../common.jsx'
import { OFFSET, locationFrom, placing } from '../../model/places.js'

/**
 * A balise or magnet of train protection: its system and kind, with the
 * settings the catalogue declares for them; where it acts, the signal it
 * belongs to, how it acts. Where it must lie to its signal is not checked yet.
 */
export default function BaliseEditor({ balise: chosen, domain, net, findings, ops, setMode, readOnly, t, language }) {
  const signals = domain.collections.signals
  const signalOptions = signals.filter(s => s.kind === 'real').map(s => ({ value: s.id, label: labelOf('signals', s, t) }))
    .sort((a, b) => String(a.label).localeCompare(String(b.label), 'de', { numeric: true }))
  const set = (patch) => ops.update('balises', { ...chosen, ...patch })
  /** A system or kind changed: the settings follow what the new kind asks and allows. */
  const retype = (system, kind) => {
    const settings = baliseSettings(system, kind, chosen.settings)
    set({ system, kind, settings: Object.keys(settings).length ? settings : undefined })
  }
  return (
    <div className="tool-editor">
      <EditorHead title={[chosen.name, baliseWord(chosen, language, t)].filter(Boolean).join(' · ')} readOnly={readOnly} t={t} onDelete={() => ops.remove('balises', chosen.id)} />
      <ObjectFindings findings={findings} collection="balises" id={chosen.id} t={t} />
      <div className="ui-fields">
        <TextField compact wide label={t('name')} value={chosen.name ?? ''} disabled={readOnly} onChange={(v) => set({ name: v || undefined })} />
        <SelectField compact label={t('protection_system')} value={chosen.system} options={protectionSystemOptions(language)} disabled={readOnly}
          onChange={(v) => retype(v, Object.keys(protectionSystemOf(v)?.arten ?? {})[0])} />
        <SelectField compact label={t('kind')} value={chosen.kind} options={baliseKindOptions(chosen.system, language)} disabled={readOnly}
          onChange={(v) => retype(chosen.system, v)} />
        <SelectField compact wide label={t('belongs_to')} value={chosen.signalId ?? null} empty={t('none')} options={signalOptions} disabled={readOnly} onChange={(v) => set({ signalId: v ?? undefined })} />
        <SelectField compact wide label={t('effect')} value={chosen.effect} options={effectOptions(language)} disabled={readOnly} onChange={(v) => set({ effect: v })} />
      </div>
      <SettingsFields balise={chosen} readOnly={readOnly} t={t} language={language}
        onChange={(settings) => set({ settings: Object.keys(settings).length ? settings : undefined })} />
      <Section title={t('location')}>
        <LocationFields loc={chosen.at} net={net} t={t} readOnly={readOnly} onChange={(at) => set({ at })}
          onPlace={() => setMode(placing('balises', (hit) => ({ ...chosen, at: locationFrom(hit, { offset: chosen.at?.offset ?? OFFSET.balises }) }), (b) => ops.update('balises', b)))} />
      </Section>
    </div>
  )
}

/** The settings a balise's kind allows, each as the catalogue declares it: a choice, a number with its unit, a flag, a text. */
function SettingsFields({ balise, onChange, readOnly, t, language }) {
  const declared = protectionSystemOf(balise.system)?.einstellungen ?? {}
  const kind = baliseKindOf(balise.system, balise.kind)
  if (!kind?.erlaubt.length) return null
  const values = balise.settings ?? {}
  const put = (name, v) => {
    const next = { ...values }
    if (v === undefined || v === null || v === '') delete next[name]
    else next[name] = v
    onChange(next)
  }
  return (
    <Section title={t('settings')}>
      <div className="ui-fields">
        {kind.erlaubt.map(name => {
          const s = declared[name]
          if (!s) return null
          const label = `${titleIn(s, language)}${s.einheit ? ` [${s.einheit}]` : ''}`
          const must = kind.pflicht.includes(name)
          switch (s.typ) {
            case 'auswahl':
              return <SelectField key={name} compact wide label={label} value={values[name] ?? null} empty={must ? undefined : t('none')} disabled={readOnly}
                options={Object.entries(s.werte).map(([value, w]) => ({ value, label: titleIn(w, language) }))} onChange={(v) => put(name, v ?? undefined)} />
            case 'zahl':
              return <NumberField key={name} compact label={label} value={values[name] ?? null} min={s.min} max={s.max} disabled={readOnly} onChange={(v) => put(name, v ?? undefined)} />
            case 'wahrheitswert':
              return <div key={name} className="ui-wide"><Checkbox label={label} checked={values[name] === true} disabled={readOnly} onChange={(v) => put(name, v || undefined)} /></div>
            default:
              return <TextField key={name} compact label={label} value={values[name] ?? ''} disabled={readOnly} onChange={(v) => put(name, v)} />
          }
        })}
      </div>
    </Section>
  )
}
