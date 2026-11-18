import { useState } from 'react'
import { exportProjectDocument } from '../../../storage'
import { PayloadError } from '@rail-across/core/project'
import { CONTAINER_EXTENSION, projectFile } from '@rail-across/core/document'
import { downloadContainer } from '../../../utils/common'
import { useI18n } from '../../../locales/i18nContext'
import { useProject, useTracks } from '../../../hooks/useStore'
import MessageList from '../../form/MessageList'
import ExchangeSection from './ExchangeSection'

/**
 * The whole project as a file rail-across-1.0 in its container (.railacross;
 * decision 0002). A project the document's schemas refuse is not written at
 * first, and what they found is shown — a click that does nothing would leave
 * the user guessing. It can then be written anyway, its document as it stands
 * in the container, to be looked at or mended elsewhere; the import will
 * refuse it as it stands.
 */
export default function ProjectSection() {
  const { t, fill } = useI18n()
  const project = useProject()
  const tracks = useTracks()
  const [errors, setErrors] = useState([])

  const exportIt = ({ check = true } = {}) => {
    setErrors([])
    try {
      const doc = exportProjectDocument({ check })
      const file = check ? projectFile({ infra: doc }, { savedAt: new Date().toISOString() }) : doc
      downloadContainer(file, `${project.title || 'project'}${CONTAINER_EXTENSION}`)
    } catch (err) {
      setErrors([t('data_exchange_export_failed'), ...(err instanceof PayloadError ? err.details.slice(0, 5) : [err.message])])
    }
  }

  return (
    <ExchangeSection title={t('data_exchange_project')}
      description={fill('data_exchange_project_desc', { tracks: tracks.length })}>
      <button className="ui-btn ui-btn-primary ui-btn-block" disabled={!project} onClick={() => exportIt()}>
        {t('data_exchange_export')}
      </button>
      <MessageList items={errors} />
      {errors.length > 0 && (
        <button className="ui-btn ui-btn-block mt-2" onClick={() => exportIt({ check: false })}>
          {t('data_exchange_export_anyway')}
        </button>
      )}
    </ExchangeSection>
  )
}
