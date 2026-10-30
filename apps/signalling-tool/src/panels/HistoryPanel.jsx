import { useMemo } from 'react'
import { Panel } from '@rail-across/ui'
import { stepText, stepsOf } from '../model/history.js'

/**
 * The history, as the Layout-Tool has it: opened by the button between the
 * undo and the redo arrow. Every step that can be undone or redone — what is
 * ahead (redo) on top, greyed, the state now marked, what lies behind below
 * it, and at the foot where the history starts. A click goes there in one
 * move and the panel stays open.
 */
export default function HistoryPanel({ history, travel, t }) {
  const { past, future } = useMemo(() => stepsOf(history), [history])
  const empty = !past.length && !future.length
  return (
    <Panel title={t('step_history_title')} intro={t('step_history_intro')}>
      {empty ? <p className="ui-list-empty">{t('step_history_empty')}</p> : (
        <div className="step-history-list">
          {[...future].reverse().map((step, i) => (
            <button key={`f${i}`} type="button" className="step-history-item step-history-ahead" onClick={() => travel(future.length - i)}>
              {stepText(step, t)}
            </button>
          ))}
          {[...past].reverse().map((step, i) => (
            <button key={`p${i}`} type="button" aria-current={i === 0 ? 'step' : undefined}
              className={`step-history-item${i === 0 ? ' step-history-now' : ''}`} onClick={() => travel(-i)}>
              {stepText(step, t)}
            </button>
          ))}
          <button type="button" aria-current={past.length ? undefined : 'step'}
            className={`step-history-item step-history-start${past.length ? '' : ' step-history-now'}`} onClick={() => travel(-past.length)}>
            {t('step_history_start')}
          </button>
        </div>
      )}
    </Panel>
  )
}
