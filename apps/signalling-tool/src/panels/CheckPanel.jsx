import { Badge, Message, Panel, Section } from '@rail-across/ui'
import { labelOf } from '../model/domain.js'

/**
 * What the core's check says (checkSignalling): the errors, then the
 * warnings, each with the object it is about; a click shows that object in
 * its panel and on the plan.
 */
export default function CheckPanel({ domain, findings, select, t }) {
  const signalName = (id) => labelOf('signals', domain.collections.signals.find(s => s.id === id), t)
  const name = (f) => {
    const obj = domain.collections[f.collection]?.find(o => o.id === f.id)
    if (f.collection === 'routes' && obj && !obj.name) return `${signalName(obj.startSignalId)} → ${signalName(obj.targetSignalId)}`
    if (f.collection === 'overlaps' && obj) return `${signalName(obj.signalId)} ${obj.name ?? ''}`.trim()
    return labelOf(f.collection, obj, t) ?? t(`collection_${f.collection}`)
  }
  const row = (f, tone) => (
    <li key={f.key}>
      <button type="button" className="tool-finding-row" onClick={() => select({ collection: f.collection, id: f.id })}>
        <Badge tone={tone === 'error' ? 'danger' : 'warn'}>{t(`collection_${f.collection}`)}</Badge>
        <span className="tool-finding-text"><strong>{name(f)}</strong> — {t(`finding_${f.code}`, f.params)}</span>
      </button>
    </li>
  )
  const { errors, warnings } = findings
  return (
    <Panel title={t('section_check')} intro={t('section_check_intro')}>
      {!errors.length && !warnings.length ? <Message tone="ok">{t('check_clean')}</Message>
        : <p className="ui-muted">{t('check_counts', { e: errors.length, w: warnings.length })}</p>}
      {errors.length > 0 && <Section title={`${t('errors')} (${errors.length})`}><ul className="tool-finding-list">{errors.map(f => row(f, 'error'))}</ul></Section>}
      {warnings.length > 0 && <Section title={`${t('warnings')} (${warnings.length})`}><ul className="tool-finding-list">{warnings.map(f => row(f, 'warn'))}</ul></Section>}
    </Panel>
  )
}
