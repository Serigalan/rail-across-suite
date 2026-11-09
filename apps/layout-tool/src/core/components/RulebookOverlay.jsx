import { useEffect, useState } from 'react'
import { optimizerRulebooks } from '../utils/optimizer'
import { BUNDLED_CATALOGUE_VERSION, LIMIT_SEVERITIES, optimizerLimitRows, CATALOG_ID } from '@rail-across/core/rules'
import { SWITCH_RULEBOOK } from '@rail-across/core/switch'
import SwitchRulebook from './SwitchRulebook'
import CrossSectionRulebook from './CrossSectionRulebook'
import { CROSS_SECTION_CATALOGUE } from '@rail-across/core/crossSection'
import RuleCatalogueView from './RuleCatalogueView'
import { useI18n } from '../locales/i18nContext'
import CloseButton from './form/CloseButton'

/**
 * The rulebooks a layout is held to, read-only, in the same popup shell the
 * track editor uses. A selector beside the heading names them; the page below
 * is whichever one is chosen.
 *
 * **One rulebook is one entry**, however many faces it has. DB Ril 800.0110 is
 * the rules the app bundles (`ruleCatalogue.js`) and the very file the optimizer
 * applies — so it is listed once, the rules first, and under them the limits
 * the optimizer module (decision 0019 of the suite) says a run is held to at
 * Regelwert and Ermessensgrenze, read out of the same file.
 */

const BUNDLED_IDS = [CATALOG_ID, SWITCH_RULEBOOK.id, CROSS_SECTION_CATALOGUE.katalog.id]

export default function RulebookOverlay({ regelwerkId, onClose }) {
  const { t, fill } = useI18n()
  // The optimizer module's catalogue: { list() → { rulebooks, drift }, get(id) → regelwerk | null }.
  const service = optimizerRulebooks
  // null while the list is still being asked for, [] once the service has
  // answered with nothing — the two read the same in a table but not to the
  // reader, who is told either "loading" or "no server".
  const [rulebooks, setRulebooks] = useState(service ? null : [])
  const [wanted, setWanted] = useState(regelwerkId ?? '')
  // Keyed by id, and only ever written from the fetch callback — never
  // synchronously in the effect body — so a change of id does not need its own
  // reset: stale content for a previous id is simply not `current` and reads
  // as still loading until its own fetch answers.
  const [status, setStatus] = useState(null)   // { id, regelwerk } | { id, failed: true }

  useEffect(() => {
    if (!service) return undefined
    let cancelled = false
    service.list().then(({ rulebooks: list }) => {
      if (!cancelled) setRulebooks(list)
    })
    return () => { cancelled = true }
  }, [service])

  // The bundled rulebooks say their own name through the app, in whichever
  // language it is asked in (regelwerk_title_<id>) — the way every other name
  // a catalogue states about itself is spoken (see switchKindLabelKey). A
  // served rulebook has no such key and is named as the service states it.
  const all = [
    ...BUNDLED_IDS.map(id => ({ id, name: t(`regelwerk_title_${id}`) })),
    ...(rulebooks ?? [])
      .filter(rw => !BUNDLED_IDS.includes(rw.id))
      .map(rw => ({ id: rw.id, name: rw.name })),
  ]
  const id = wanted || CATALOG_ID
  // Only what the service actually lists is asked for — a purely bundled
  // rulebook has nothing to fetch, and a fetch for it would only ever fail.
  const served = !!rulebooks?.some(rw => rw.id === id)

  useEffect(() => {
    if (!id || !served) return
    let cancelled = false
    service.get(id).then(rw => {
      if (cancelled) return
      setStatus(rw ? { id, regelwerk: rw } : { id, failed: true })
    })
    return () => { cancelled = true }
  }, [id, served, service])

  const current = status?.id === id ? status : null
  const regelwerk = current?.regelwerk ?? null
  const failed = !!current?.failed
  const bundled = BUNDLED_IDS.includes(id)

  // A limit is a number, a range, a list of steps, the transition forms a run
  // may hand out, or the worst severity it may produce — each read as itself.
  const limitText = (row, value) => {
    if (value === null || value === undefined) return '—'
    if (row.kind === 'forms') return value.map(form => t(`transition_type_${form}`)).join(', ') || '—'
    if (row.kind === 'severity') return t(`rule_sev_${value}`)
    const text = Array.isArray(value) ? value.join(row.join) : String(value)
    return row.unit ? `${text} ${row.unit}` : text
  }

  return (
    <div className="track-table-overlay constraints-overlay">
      <div className="track-table-header">
        <span className="track-table-title constraints-title">
          {t('constraints_regelwerk')}
          {/* Beside the heading, not out by the close button: it says which
              regelwerk the heading is about, so it belongs to the heading. */}
          <select className="track-table-input constraints-select"
            value={id} onChange={e => setWanted(e.target.value)}>
            {all.map(rw => <option key={rw.id} value={rw.id}>{rw.name}</option>)}
          </select>
          <span className="track-table-subtitle">{t('constraints_readonly')}</span>
        </span>
        <CloseButton onClick={onClose} />
      </div>

      <div className="track-table-scroll constraints-scroll">
        {/* A service that cannot be asked is said once, above whatever is
            shown: the bundled rules are still there and still true, and the
            reader has to know that the values half is missing rather than
            gone. */}
        {service && rulebooks?.length === 0 && (
          <p className="constraints-error">{t('constraints_service_down')}</p>
        )}

        {id === CATALOG_ID && <RuleCatalogueView />}
        {id === SWITCH_RULEBOOK.id && <SwitchRulebook />}
        {id === CROSS_SECTION_CATALOGUE.katalog.id && <CrossSectionRulebook />}

        {/* Three states, said apart: still asking, asked and no server, and
            the table itself. A panel that needs the service says so rather
            than showing an empty table. */}
        {served && !failed && !regelwerk && (
          <p className="constraints-hint">{t('constraints_loading')}</p>
        )}
        {failed && <p className="constraints-error">{t('optimizer_missing')}</p>}
        {!bundled && !served && rulebooks !== null && (
          <p className="constraints-error">{t('optimizer_missing')}</p>
        )}

        {regelwerk && (
          <>
            {/* Under the rules: what the optimizer says it runs at under them,
                per level, beside the app's own reading of the same file. */}
            <h4 className="constraints-subsection">{t('constraints_optimizer_values')}</h4>
            <p className="constraints-hint">
              {regelwerk.name} · {t('constraints_katalog_revision')} {regelwerk.version}
            </p>
            {regelwerk.version !== BUNDLED_CATALOGUE_VERSION && (
              <p className="constraints-error">
                {fill('constraints_version_mismatch', { served: regelwerk.version, bundled: BUNDLED_CATALOGUE_VERSION })}
              </p>
            )}
            <table className="track-table constraints-table">
              <thead>
                <tr>
                  <th />
                  <th>{t('optimize_grenzwert_reg')}</th>
                  <th>{t('optimize_grenzwert_discretion')}</th>
                </tr>
              </thead>
              <tbody>
                {optimizerLimitRows(regelwerk).map(row => (
                  <tr key={row.label}>
                    <td>{t(row.label)}</td>
                    {Object.keys(LIMIT_SEVERITIES).map(level => {
                      const cell = row[level]
                      return (
                        <td key={level} className="constraints-value">
                          {limitText(row, cell.served ?? cell.app)}
                          {/* Only where the two disagree — a service deployed
                              from another commit than this bundle. */}
                          {cell.mismatch && (
                            <span className="constraints-app-mismatch">
                              {t('constraints_app_mismatch')}: {limitText(row, cell.app)}
                            </span>
                          )}
                        </td>
                      )
                    })}
                  </tr>
                ))}
              </tbody>
            </table>
          </>
        )}
      </div>
    </div>
  )
}
