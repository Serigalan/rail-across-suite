import { useEffect, useMemo, useRef, useState } from 'react'
import DayCalendar from './stagePicker/DayCalendar'
import { useI18n } from '../locales/i18nContext'
import { useMapStage, useProject } from '../hooks/useStore'
import { setMapStage } from '../storage'
import { dayLabel, isDayKey, stageLabel, stageOptions } from '../utils/stage'

/**
 * The point in time the map shows (decision 0007): every stage at once, as
 * planned — the default —, or what is in service on a day, picked in a
 * calendar that marks the days something changes on; the existing and the
 * final state beside it, and those days as shortcuts. Beside the working
 * copy's bar, as an overlay of the same kind; the choice opens as a dropdown.
 */
export default function StagePicker() {
  const { t, language } = useI18n()
  const project = useProject()
  const stage = useMapStage()
  const options = useMemo(() => stageOptions(project), [project])
  const [open, setOpen] = useState(false)
  const root = useRef(null)

  // Closed by a click elsewhere or Escape.
  useEffect(() => {
    if (!open) return
    const away = (e) => { if (!root.current?.contains(e.target)) setOpen(false) }
    const esc = (e) => { if (e.key === 'Escape') setOpen(false) }
    document.addEventListener('pointerdown', away)
    document.addEventListener('keydown', esc)
    return () => { document.removeEventListener('pointerdown', away); document.removeEventListener('keydown', esc) }
  }, [open])

  if (!project) return null
  const pick = (key) => { setMapStage(key); setOpen(false) }
  const current = stage == null ? t('stage_all')
    : stage === 'existing' ? t('stage_existing')
    : stage === 'final' ? t('stage_final')
    : dayLabel(stage, language)
  const days = options.filter(o => o.date)
  const marked = new Set(days.map(o => o.key))

  return (
    <div className="stage-picker" ref={root}>
      <span>{t('stage_picker')}</span>
      <button type="button" title={t('stage_picker_hint')} className={`stage-picker-toggle${stage != null ? ' stage-picker-set' : ''}`}
        aria-haspopup="dialog" aria-expanded={open} onClick={() => setOpen(o => !o)}>
        {current} <span aria-hidden="true">▾</span>
      </button>
      {open && (
        <div className="stage-picker-pop" role="dialog" aria-label={t('stage_picker')}>
          <DayCalendar value={isDayKey(stage) ? stage : null} start={days[0]?.key} marked={marked}
            onPick={(day) => pick(day)} />
          <div className="stage-picker-states">
            <button type="button" aria-pressed={stage == null} onClick={() => pick(null)}>{t('stage_all')}</button>
            <button type="button" aria-pressed={stage === 'existing'} onClick={() => pick('existing')}>{t('stage_existing')}</button>
            <button type="button" aria-pressed={stage === 'final'} onClick={() => pick('final')}>{t('stage_final')}</button>
          </div>
          {days.length > 0 && (
            <div className="stage-picker-days">
              <span>{t('stage_days')}</span>
              {days.map(o => (
                <button key={o.key} type="button" aria-pressed={stage === o.key} onClick={() => pick(o.key)}>
                  {stageLabel(o, t, language)}
                </button>
              ))}
            </div>
          )}
          <p className="stage-picker-note">{t('stage_day_note')}</p>
        </div>
      )}
    </div>
  )
}
