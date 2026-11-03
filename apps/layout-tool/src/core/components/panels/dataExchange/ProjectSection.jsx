import { useState } from 'react'
import { exportProjectDocument } from '../../../storage'
import { PayloadError } from '@rail-across/core/project'
import { downloadJSON } from '../../../utils/common'
import { useI18n } from '../../../locales/i18nContext'
import { useProject, useTracks } from '../../../hooks/useStore'
import MessageList from '../../form/MessageList'
import ExchangeSection from './ExchangeSection'

/**
 * The whole project as a file: its core 1.0 document (.railacross.json). A
 * project the document's schemas refuse is not written, and what they found
 * is shown — a click that does nothing would leave the user guessing.
 */
export default function ProjectSection() {
  const { t, fill } = useI18n()
  const project = useProject()
  const tracks = useTracks()
  const [errors, setErrors] = useState([])

  const exportIt = () => {
    setErrors([])
    try {
      downloadJSON(exportProjectDocument(), `${project.title || 'project'}.railacross.json`)
    } catch (err) {
      setErrors([t('data_exchange_export_failed'), ...(err instanceof PayloadError ? err.details.slice(0, 5) : [err.message])])
    }
  }

  return (
    <ExchangeSection title={t('data_exchange_project')}
      description={fill('data_exchange_project_desc', { tracks: tracks.length })}>
      <button className="ui-btn ui-btn-primary ui-btn-block" disabled={!project} onClick={exportIt}>
        {t('data_exchange_export')}
      </button>
      <MessageList items={errors} />
    </ExchangeSection>
  )
}
