import RoutesForm from './EditElementPanel/RoutesForm'
import InfoTip from '../form/InfoTip'
import { useI18n } from '../../locales/i18nContext'

/** The Civil-Tool's named runs of tracks (decision 0025): the Layout-Tool shows them in its profiles and tables. */
export default function CivilRunsPanel() {
  const { t } = useI18n()
  return (
    <>
      <h2>{t('routes')}<InfoTip text={t('route_hint')} /></h2>
      <RoutesForm />
    </>
  )
}
