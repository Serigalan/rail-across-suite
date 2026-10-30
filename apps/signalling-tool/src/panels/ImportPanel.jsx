import { useState } from 'react'
import { Button, Field, Message, Panel, Section, UploadIcon } from '@rail-across/ui'

/**
 * Taking a PlanPro plan's signalling: the file is read in the browser, its
 * signalling laid on the project's net, and what came of it shown — how
 * much stands on a track, how much is orphaned, what kinds of objects the
 * plan has that this model does not hold. Replacing the domain is one step;
 * checking in makes it last.
 */
export default function ImportPanel({ record, domain, ops, setNotice, setSection, t }) {
  const [read, setRead] = useState(null)
  const [error, setError] = useState(null)
  const [busy, setBusy] = useState(false)

  const onFile = async (file) => {
    if (!file) return
    setBusy(true); setError(null); setRead(null)
    try {
      const bytes = new Uint8Array(await file.arrayBuffer())
      // The reader and the unzipping come when first needed; a large plan takes a moment.
      const { importPlanPro } = await import('../model/planproFile.js')
      setRead({ ...importPlanPro(bytes, record), file: file.name })
    } catch (err) {
      setError(err.message ?? String(err))
    } finally {
      setBusy(false)
    }
  }
  const c = read?.signalling.collections
  const title = read ? (read.header.title ?? read.header.subInstallation ?? read.file) : null
  const had = Object.values(domain.collections).some(l => l.length)

  return (
    <Panel title={t('section_import')} intro={t('section_import_intro')}>
      <Field label={t('import_choose')}>
        <input type="file" accept=".planpro,.ppxml,.xml" disabled={busy} onChange={(e) => onFile(e.target.files?.[0])} />
      </Field>
      {busy && <p className="ui-muted">{t('loading')}</p>}
      {error && <Message tone="error">{t('import_failed', { error })}</Message>}
      {read && (
        <Section title={t('import_read', { title })}>
          <p className="ui-muted">{[read.header.installation, read.header.tool, read.header.xsdVersion && `XSD ${read.header.xsdVersion}`].filter(Boolean).join(' · ')}</p>
          <p>{t('import_counts', { signals: c.signals.length, routes: c.routes.length, sections: c.trackSections.length, pzb: c.pzbElements.length })}</p>
          {read.sameNet ? <Message tone="ok">{t('import_same_net')}</Message>
            : <Message tone={read.counts.orphaned ? 'warn' : 'ok'}>{t('import_placed', read.counts)}</Message>}
          {Object.keys(read.report.skipped).length > 0 && (
            <p className="ui-muted">{t('import_skipped', { list: Object.entries(read.report.skipped).map(([k, n]) => `${k} (${n})`).join(', ') })}</p>
          )}
          {had && <p className="ui-muted">{t('import_replace_hint')}</p>}
          <div className="ui-row">
            <Button variant="primary" icon={<UploadIcon size={16} />}
              onClick={() => { ops.replaceAll(read.signalling); setNotice({ tone: 'ok', text: t('import_done', { title }) }); setRead(null); setSection('check') }}>
              {t('import_replace')}
            </Button>
          </div>
        </Section>
      )}
    </Panel>
  )
}
