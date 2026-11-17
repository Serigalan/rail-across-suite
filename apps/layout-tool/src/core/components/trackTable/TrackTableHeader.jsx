import { SPEED_STEP } from '@rail-across/core/rules'
import { useI18n } from '../../locales/i18nContext'
import { lengthText } from './rowText'
import CloseButton from '../form/CloseButton'
import NumberInput from '../form/NumberInput'

/**
 * The bar above the table: the track and its length, the days it is in
 * service (`days`, a TrackDays — none for a route), the line speed and the
 * button that fills the speed column up to it, the one that merges alike
 * elements, what the edits reached (or why the last one was refused), Save and
 * close.
 */
export function TrackTableBar({ title, length, days, cap, onCap, onMaxSpeeds, onMerge, reachError, notice, reached, dirty, changed, onSave, onClose }) {
  const { t, fill, language } = useI18n()
  return (
    <div className="track-table-header">
      <span className="track-table-title">
        {title}
        <span className="track-table-subtitle">{lengthText(length, language)} m</span>
      </span>
      {days}
      <div className="row-wide">
        <label className="track-table-cap" title={t('table_speed_cap_hint')}>
          {t('table_speed_cap')}
          <NumberInput className="track-table-input track-table-cap-input" min="0" step={SPEED_STEP}
            placeholder="–" value={cap} onChange={e => onCap(e.target.value)} />
        </label>
        <button className="track-table-vmax-btn" onClick={onMaxSpeeds} title={t('table_set_max_speeds_hint')}>
          {t('table_set_max_speeds')}
        </button>
        <button className="track-table-vmax-btn" onClick={onMerge} title={t('table_merge_hint')}>
          {t('table_merge')}
        </button>
        {reachError && <span className="track-table-reach-error">{t(reachError)}</span>}
        {!reachError && notice && (
          <span className="track-table-reach">{typeof notice === 'string' ? t(notice) : fill(notice.key, notice.vars)}</span>
        )}
        {!reachError && !notice && reached.trackIds.length > 1 && (
          <span className="track-table-reach">
            {fill('table_edit_reach', { tracks: String(reached.trackIds.length), switches: String(reached.switchIds.length) })}
          </span>
        )}
        {/* Nothing here is written until this is pressed, so it says whether
            anything is waiting — and on how many tracks, since an edit reaches
            past the one on screen. */}
        <button className={`track-table-save-btn${dirty ? ' track-table-save-btn-dirty' : ''}`}
          onClick={onSave} disabled={!dirty}
          title={dirty ? fill('table_unsaved', { tracks: String(changed) }) : undefined}>
          {dirty ? `${t('btn_save')} •` : t('btn_save')}
        </button>
        <CloseButton onClick={onClose} />
      </div>
    </div>
  )
}

/**
 * The days a whole track is in service (decision 0007): its lifespan's
 * first and last, each a browser date field (ISO 8601), empty where the
 * lifespan states none or "open". They are the track's and not an element's,
 * so they stand in the table's head and not in a column. `onChange(key, day)`
 * with 'from' or 'to', `day` undefined where the field was emptied.
 */
export function TrackDays({ life = {}, onChange }) {
  const day = (v) => (typeof v === 'string' && /^\d{4}-\d{2}-\d{2}$/.test(v) ? v : '')
  const { t } = useI18n()
  const field = (key, word, label, hint) => (
    <label className="track-table-day" title={t(hint)}>
      {t(word)}
      <input type="date" className="track-table-input track-table-input-day" value={day(life[key])} aria-label={t(label)}
        onChange={e => onChange(key, e.target.value || undefined)} />
    </label>
  )
  return (
    <span className="track-table-days">
      <span className="track-table-days-lead">{t('table_in_service')}</span>
      {field('from', 'table_in_service_from_word', 'table_in_service_from', 'table_in_service_from_hint')}
      {field('to', 'table_in_service_to_word', 'table_in_service_to', 'table_in_service_to_hint')}
    </span>
  )
}

/** The column heads. */
export function TrackTableHead() {
  const { t } = useI18n()
  return (
    <thead>
      <tr>
        <th>#</th>
        <th title={t('table_rules_hint')}>{t('table_rules')}</th>
        <th title={t('table_station_hint')}>{t('table_station')} (m)</th>
        <th>{t('table_type')}</th>
        <th title={t('table_role_hint')}>{t('table_role')}</th>
        <th>{t('table_bearing')} (°)</th>
        <th>{t('table_end_bearing')} (°)</th>
        <th>{t('table_length')} (m)</th>
        <th>{t('table_radius')} (m)</th>
        <th>{t('table_speed')} (km/h)</th>
        <th>{t('table_cant')} (mm)</th>
        <th title={t('table_cant_ends_hint')}>{t('table_cant_start')} (mm)</th>
        <th title={t('table_cant_ends_hint')}>{t('table_cant_end')} (mm)</th>
        <th>{t('table_cant_def')} (mm)</th>
        <th title={t('table_max_speed_hint')}>
          {t('table_max_speed')} (km/h)
        </th>
        <th title={t('table_crs_hint')}>{t('table_crs')}</th>
        <th title={t('table_comparison_radius_hint')}>{t('table_comparison_radius')} (m)</th>
      </tr>
    </thead>
  )
}
