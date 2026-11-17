import { useState } from 'react'
import { useI18n } from '../../locales/i18nContext'
import ExchangeSection from './dataExchange/ExchangeSection'
import ProjectSection from './dataExchange/ProjectSection'
import CompareSection from './dataExchange/CompareSection'
import TracksSection from './dataExchange/TracksSection'
import VermEsnSection from './dataExchange/VermEsnSection'
import OsrdSection from './dataExchange/OsrdSection'
import CsvSection from './dataExchange/CsvSection'
import { extensionsOf } from '../../extensions'
import AlignmentSection from './dataExchange/AlignmentSection'
import SpeedRangesSection from './dataExchange/SpeedRangesSection'
import ImportReports from './dataExchange/ImportReports'
import useImportReports from './dataExchange/useImportReports'
import ProviImportSection from './ProviImportSection'
import BackButton from './BackButton'
import { RailJsonIcon, TracksExchangeIcon, VermEsnIcon } from '../icons'
import { EDITS_CIVIL } from '../../tool'

// The formats of every session, each a page of its own, as the other panels' options —
// in the Civil-Tool (decision 0008) the one that brings a civil object: Verm.ESN's
// axis as a reference axis; what brings tracks is the Layout-Tool's.
const FORMATS = [
  { id: 'tracks', icon: TracksExchangeIcon, titleKey: 'data_exchange_tracks', Component: TracksSection },
  { id: 'vermesn', icon: VermEsnIcon, titleKey: 'data_exchange_vermesn', Component: VermEsnSection, civil: true },
  { id: 'osrd', icon: RailJsonIcon, titleKey: 'data_exchange_osrd', Component: OsrdSection },
].filter(f => !EDITS_CIVIL || f.civil)

/**
 * Data exchange: one section per format (R5.1). The project's export stands
 * at the head; the formats of every session are options below it, each a page
 * of its own — Tracks, Verm.ESN, RailJSON, and a format the server converts
 * that names itself an option (`importSections` with `menu`, Paket L: the MDB
 * import). Behind the dot at the bottom are those needed now and then — the
 * comparison with a file, Provi, the Gleislage CSV, the other formats the
 * server converts, the reports they leave, the alignment exchange format, and
 * the highest speeds along a run in ranges, as CSV or SVG.
 */
export default function DataExchangePanel({ onShowCompare }) {
  const { t } = useI18n()
  const [page, setPage] = useState(null)
  const [more, setMore] = useState(false)
  const reports = useImportReports()
  const converted = extensionsOf('importSections')
  const options = [...FORMATS, ...(EDITS_CIVIL ? [] : converted.filter(x => x.menu).map(x => ({ ...x, ...x.menu, report: true })))]

  const open = options.find(o => o.id === page)
  if (open) {
    const Page = open.Component
    return (
      <>
        <BackButton onBack={() => setPage(null)} />
        {open.report ? <>
          <Page onReport={reports.add} />
          <ImportReports reports={reports.reports} onClear={reports.clear} />
        </> : <Page />}
      </>
    )
  }

  return (
    <>
      <h2>{t('data_exchange')}</h2>
      <ProjectSection />
      <div className="create-element-options">
        {options.map(({ id, icon: Icon, titleKey }) => (
          <button key={id} className="ui-btn ui-btn-option" onClick={() => setPage(id)}>
            <Icon />
            {t(titleKey)}
          </button>
        ))}
      </div>
      {more && (
        <>
          <CompareSection onShowCompare={onShowCompare} />
          {!EDITS_CIVIL && <>
            <ExchangeSection title={t('data_exchange_provi')} description={t('data_exchange_provi_desc')}>
              <ProviImportSection onReport={(source, counts, lines) => reports.add({ source, ...counts, lines })} />
            </ExchangeSection>
            <CsvSection />
            {converted.filter(x => !x.menu).map(({ id, Component }) => <Component key={id} onReport={reports.add} />)}
            <ImportReports reports={reports.reports} onClear={reports.clear} />
          </>}
          <AlignmentSection />
          <SpeedRangesSection />
        </>
      )}
      <div className="panel-dot-row">
        <button className={`panel-dot${more ? ' active' : ''}`} title={t('data_exchange_more')}
          aria-label={t('data_exchange_more')} aria-expanded={more} onClick={() => setMore(o => !o)} />
      </div>
    </>
  )
}
