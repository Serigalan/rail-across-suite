import { IconButton, FitIcon, Message } from '@rail-across/ui'
import { pathLength } from '@rail-across/running-times'
import RuntimeMap from './RuntimeMap.jsx'
import RunChart from './RunChart.jsx'
import RuntimeTrain from './RuntimeTrain.jsx'
import RuntimeResult from './RuntimeResult.jsx'

// Computing a running time on its own, in the Fahrplan-Editor: its state is useRuntime.js's.

/** The panel of a section of it: the train to compute (`train`), or the result. */
export function RuntimePanel({ section, panelProps }) {
  return section === 'result' ? <RuntimeResult {...panelProps} /> : <RuntimeTrain {...panelProps} />
}

/** Its workspace: the map with the train's path, its places and the train played, the speed and the height under it. */
export function RuntimeView({ bar, viewProps: v }) {
  const { t } = v
  return (
    <div className="sim-stack">
      <div className="sim-map-wrap">
        {bar}
        <div className="tool-fit"><IconButton label={t('fit')} icon={<FitIcon />} onClick={() => v.setFitKey(k => k + 1)} /></div>
        {v.picking && (
          <div className="tool-mode" role="status">
            <span>{t(`picking_${v.picking}`)}</span>
            <button type="button" className="ui-btn ui-btn-small" onClick={() => v.setPicking(null)}>{t('cancel')}</button>
          </div>
        )}
        {!v.record.tracks?.length && <div className="tool-notice"><Message tone="info">{t('empty_net')}</Message></div>}
        <RuntimeMap record={v.record} net={v.net} path={v.route?.path ?? null} points={v.mapPoints} train={v.train} picking={v.picking} onPick={v.pickOnMap} fitKey={v.fitKey} t={t} />
      </div>
      {v.way && <RunChart run={v.ok} profile={v.profile} length={v.ok ? v.ok.request.path.length : pathLength(v.way.path)} offsets={v.way.offsets} labels={v.way.labels} at={v.at} t={t} language={v.language} />}
    </div>
  )
}
