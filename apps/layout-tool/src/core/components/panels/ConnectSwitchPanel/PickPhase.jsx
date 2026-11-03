import { useI18n } from '../../../locales/i18nContext'
import CancelButton from '../../form/CancelButton'

/** What the dialog shows while the tracks are picked. */
export default function PickPhase({ phase, picks, pickStatus, onCancel }) {
  const { t } = useI18n()
  return (
    <>
      <p>{phase === 'select_first' ? t('scurve_hint_first') : t('scurve_hint_second')}</p>
      {picks.length > 0 && (
        <p className="msg-info">
          {t('scurve_line1')}: {picks[0].name}
        </p>
      )}
      {pickStatus && (
        <p className={pickStatus.error ? 'msg-error' : 'msg-hint'}>
          {pickStatus.msg}
        </p>
      )}
      {phase === 'select_second' && (
        <CancelButton className="ui-btn ui-btn-block mt-8" onClick={onCancel} />
      )}
    </>
  )
}
