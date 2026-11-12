import { useMemo } from 'react'
import { useI18n } from '../locales/i18nContext'
import { useMapStage, useProject } from '../hooks/useStore'
import { setMapStage } from '../storage'
import { stageLabel, stageOptions } from '../utils/stage'

/**
 * Which construction stage the map shows (decision 0027): every one at once,
 * as planned — the default —, the existing state, each day something is put
 * into or taken out of service, the final state. Beside the working copy's
 * bar, as an overlay of the same kind.
 */
export default function StagePicker() {
  const { t, language } = useI18n()
  const project = useProject()
  const stage = useMapStage()
  const options = useMemo(() => stageOptions(project), [project])
  if (!project) return null
  return (
    <label className="stage-picker" title={t('stage_picker_hint')}>
      <span>{t('stage_picker')}</span>
      <select value={stage ?? ''} onChange={e => setMapStage(e.target.value || null)}>
        <option value="">{t('stage_all')}</option>
        {options.map(o => <option key={o.key} value={o.key}>{stageLabel(o, t, language)}</option>)}
      </select>
    </label>
  )
}
