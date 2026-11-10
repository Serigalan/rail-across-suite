import { useState } from 'react'
import { Button, Message, Option, Options, OptionsHeading, Section, TextField, DownloadIcon } from '@rail-across/ui'
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

/** The bytes of a file in Base64 as a file the browser saves — in the desktop app into its export folder. */
function save(base64, name) {
  const bytes = Uint8Array.from(atob(base64), c => c.charCodeAt(0))
  const url = URL.createObjectURL(new Blob([bytes], { type: 'application/zip' }))
  const a = Object.assign(document.createElement('a'), { href: url, download: name })
  document.body.append(a)
  a.click()
  a.remove()
  setTimeout(() => URL.revokeObjectURL(url), 1000)
}

/** A file name of the project's title, without what file systems refuse. */
const fileName = (title) => `${(title || 'Projekt').replace(/[\\/:*?"<>|]+/g, '_').trim() || 'Projekt'}.planpro`

/** The planning data a PlanPro plan files itself under; empty fields are filled by the export. */
const PLANNING = ['projectNumber', 'planner', 'organisation', 'location', 'stage']

/** Notes of the report by collection: how many of each. */
function byCollection(notes, code) {
  const n = {}
  for (const x of notes) if (x.code === code) n[x.collection] = (n[x.collection] ?? 0) + 1
  return Object.entries(n).map(([k, v]) => `${k} (${v})`).join(', ')
}

/**
 * Writing the project as a PlanPro 1.10 plan (`.planpro`, the container of
 * PlanPro tools): the net, the switches and end marks, the platforms,
 * heights and cant, and the signalling as it stands in the editor — what is
 * not checked in yet included. The planning data say who planned it; what
 * PlanPro requires and the project does not say, and what PlanPro has no
 * place for, is listed after writing.
 */
function ExportPage({ record, domain, back, t }) {
  const [written, setWritten] = useState(null)
  const [error, setError] = useState(null)
  const [planning, setPlanning] = useState({})

  const write = () => {
    setError(null)
    try {
      const given = Object.fromEntries(PLANNING.map(k => [k, planning[k]?.trim()]).filter(([, v]) => v))
      const out = writePlanPro(record, { signalling: domain, now: new Date().toISOString(), planning: given })
      save(out.archive, fileName(record.title))
      setWritten(out)
    } catch (err) {
      setWritten(null)
      setError(err.message ?? String(err))
    }
  }

  const c = written?.counts ?? {}
  const notes = written?.report.notes ?? []
  const missing = byCollection(notes, 'not_written')
  const filled = byCollection(notes, 'defaulted')
  return (
    <Panel title={t('export_planpro')} intro={t('section_export_intro')} back={back}>
      <Section title={t('export_planning')}>
        {PLANNING.map(k => (
          <TextField key={k} compact wide label={t(`export_${k}`)} value={planning[k] ?? ''}
            onChange={v => setPlanning(p => ({ ...p, [k]: v }))} placeholder={t('export_automatic')} />
        ))}
      </Section>
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
          {missing && <Message tone="warn">{t('export_not_written', { list: missing })}</Message>}
          {filled && <Message tone="info">{t('export_defaulted', { list: filled })}</Message>}
          <p className="ui-muted">{t('export_check_hint')}</p>
        </Section>
      )}
    </Panel>
  )
}
