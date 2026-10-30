import { exportProjectDocument } from '../../../storage'
import { downloadJSON } from '../../../utils/common'
import { useI18n } from '../../../locales/i18nContext'
import { useProject, useTracks } from '../../../hooks/useStore'
import ExchangeSection from './ExchangeSection'

/** The whole project as a file: its core 1.0 document (.railacross.json). */
export default function ProjectSection() {
  const { t, fill } = useI18n()
  const project = useProject()
  const tracks = useTracks()
  return (
    <ExchangeSection title={t('data_exchange_project')}
      description={fill('data_exchange_project_desc', { tracks: tracks.length })}>
      <button className="ui-btn ui-btn-primary ui-btn-block"
        disabled={!project}
        onClick={() => downloadJSON(exportProjectDocument(), `${project.title || 'project'}.railacross.json`)}>
        {t('data_exchange_export')}
      </button>
    </ExchangeSection>
  )
}
