import { useState } from 'react'
import { Button, Message, Option, Options, OptionsHeading, Section, DownloadIcon } from '@rail-across/ui'
import { writePlanPro } from '@rail-across/core/planpro'
import { Panel } from './draft.jsx'
import { ExportIcon } from '../icons.jsx'

/**
 * Data exchange, with a menu as the Layout-Tool's panel of that name: for
 * now writing the project as a PlanPro plan. There is no PlanPro import — a
 * plan holds too little of a project to be taken in.
 */
export default function ExchangePanel(props) {
  const { t } = props
  const [page, setPage] = useState('menu')
  if (page === 'planpro') return <ExportPage {...props} back={{ label: t('back'), onClick: () => setPage('menu') }} />
  return (
    <Panel title={t('section_exchange')} intro={t('section_exchange_intro')}>
      <Options>
        <OptionsHeading>PlanPro</OptionsHeading>
        <Option icon={<ExportIcon />} onClick={() => setPage('planpro')}>{t('export_planpro')}</Option>
      </Options>
    </Panel>
  )
}

/** The text as a file the browser saves — in the desktop app into its export folder. */
function save(text, name) {
  const url = URL.createObjectURL(new Blob([text], { type: 'application/xml' }))
  const a = Object.assign(document.createElement('a'), { href: url, download: name })
  document.body.append(a)
  a.click()
  a.remove()
  setTimeout(() => URL.revokeObjectURL(url), 1000)
}

/** A file name of the project's title, without what file systems refuse. */
const fileName = (title) => `${(title || 'Projekt').replace(/[\\/:*?"<>|]+/g, '_').trim() || 'Projekt'}.ppxml`

/**
 * Writing the project as a PlanPro 1.10 plan (`.ppxml`): the net, the
 * switches and end marks, and the signalling as it stands in the editor —
 * what is not checked in yet included. What PlanPro has no place for here is
 * listed after writing, by collection.
 */
function ExportPage({ record, domain, back, t }) {
  const [written, setWritten] = useState(null)
  const [error, setError] = useState(null)

  const write = () => {
    setError(null)
    try {
      const out = writePlanPro(record, { signalling: domain, now: new Date().toISOString() })
      save(out.text, fileName(record.title))
      setWritten(out)
    } catch (err) {
      setWritten(null)
      setError(err.message ?? String(err))
    }
  }

  const c = written?.counts ?? {}
  const missing = {}
  for (const n of written?.report.notes ?? []) missing[n.collection] = (missing[n.collection] ?? 0) + 1
  return (
    <Panel title={t('export_planpro')} intro={t('section_export_intro')} back={back}>
      <div className="ui-row">
        <Button variant="primary" icon={<DownloadIcon size={16} />} onClick={write}>{t('export_write')}</Button>
      </div>
      {error && <Message tone="error">{t('export_failed', { error })}</Message>}
      {written && (
        <Section title={t('export_done', { name: fileName(record.title) })}>
          <p>{t('export_counts', {
            tracks: c.TOP_Kante ?? 0, switches: c.W_Kr_Anlage ?? 0, signals: c.Signal ?? 0,
            routes: c.Fstr_Zug_Rangier ?? 0, sections: c.FMA_Anlage ?? 0,
          })}</p>
          {Object.keys(missing).length > 0 && (
            <Message tone="warn">{t('export_not_written', { list: Object.entries(missing).map(([k, n]) => `${k} (${n})`).join(', ') })}</Message>
          )}
          <p className="ui-muted">{t('export_check_hint')}</p>
        </Section>
      )}
    </Panel>
  )
}
