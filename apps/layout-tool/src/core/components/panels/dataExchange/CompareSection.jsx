import { useState } from 'react'
import { currentProject } from '../../../storage'
import { parseProjectsPayload, PayloadError } from '@rail-across/core/project'
import { fileParts, isProjectFile } from '@rail-across/core/document'
import { readProjectFileText } from '../../../utils/common'
import { useI18n } from '../../../locales/i18nContext'
import { useProject } from '../../../hooks/useStore'
import FilePickButton from '../../form/FilePickButton'
import ExchangeSection from './ExchangeSection'

/**
 * The open project against an exported file of it (AP 10.3): the file is the
 * earlier state, the project as it is now the later one. A file with several
 * projects is compared by the one with this project's id, or its first.
 */
export default function CompareSection({ onShowCompare }) {
  const { t } = useI18n()
  const project = useProject()
  const [error, setError] = useState(null)

  const compare = async (file) => {
    setError(null)
    try {
      const data = JSON.parse(await readProjectFileText(file))
      const { projects } = parseProjectsPayload(isProjectFile(data) ? fileParts(data).infra : data)
      const theirs = projects.find(p => p.id === project?.id) ?? projects[0]
      if (!theirs) { setError(t('compare_no_project')); return }
      const { image: _image, ...before } = theirs
      onShowCompare?.({
        before, after: currentProject(),
        beforeLabel: `${t('compare_file_label')} ${file.name}`, afterLabel: t('compare_current_label'),
      })
    } catch (err) {
      const code = err instanceof PayloadError ? err.code : 'invalid_payload'
      setError(t(`data_exchange_import_err_${code}`))
    }
  }

  return (
    <ExchangeSection title={t('compare_with_file')} description={t('compare_with_file_desc')}>
      <FilePickButton accept=".railacross,.json,application/json" disabled={!project} onFile={compare}>
        {t('compare_with_file')}
      </FilePickButton>
      {error && <p className="msg-error msg-small">{error}</p>}
    </ExchangeSection>
  )
}
