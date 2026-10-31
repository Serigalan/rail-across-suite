import { Button, Message, Panel, Section } from '@rail-across/ui'
import { formatDistance, formatTime } from '../format.js'

/**
 * The run: its running time — the fastest, with the margin —, the times it
 * arrives and leaves at every point, its energy at the wheels; and the run
 * played on the map and the chart, moment by moment.
 */
export default function ResultPanel({ t, language, run, stale, moment, setMoment, at }) {
  if (!run?.result) {
    return <Panel title={t('section_result')} intro={t('section_result_intro')}><p className="ui-muted">{t('no_result')}</p></Panel>
  }
  const { result, labels, request } = run
  const final = result.final, base = result.base
  const total = final.total_time
  const dwell = (request.schedule ?? []).reduce((s, x) => s + (x.stop_duration ?? 0), 0)
  const kmh = (v) => Math.round(v * 3.6)
  const points = [{ arrival: null, departure: 0 }, ...final.scheduled_points]
  return (
    <Panel title={t('section_result')} intro={t('section_result_intro')}>
      {stale && <Message tone="warn">{t('stale')}</Message>}
      <Section title={t('running_time')}>
        <dl className="sim-facts">
          <dt>{t('time_final')}</dt><dd><strong>{formatTime(total)}</strong></dd>
          <dt>{t('time_base')}</dt><dd>{formatTime(base.total_time - dwell)}</dd>
          <dt>{t('time_margin')}</dt><dd>{formatTime(result.provisional.total_time - base.total_time)}</dd>
          <dt>{t('time_dwell')}</dt><dd>{formatTime(dwell)}</dd>
          <dt>{t('distance')}</dt><dd>{formatDistance(request.path.length, language)}</dd>
          <dt>{t('mean_speed')}</dt><dd>{kmh(request.path.length / Math.max(total - dwell, 1))} km/h</dd>
          <dt>{t('energy')}</dt><dd>{(final.energy_consumption / 3.6e6).toLocaleString(language, { maximumFractionDigits: 1 })} kWh</dd>
        </dl>
      </Section>

      <Section title={t('timetable')}>
        <table className="sim-table">
          <thead><tr><th>{t('place')}</th><th>{t('arrival')}</th><th>{t('departure')}</th></tr></thead>
          <tbody>
            {points.map((p, i) => (
              <tr key={i}>
                <td>{labels[i]}</td>
                <td>{p.arrival == null ? '' : formatTime(p.arrival)}</td>
                <td>{i === points.length - 1 ? '' : formatTime(p.departure)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </Section>

      <Section title={t('play')}>
        <input type="range" className="sim-range" min={0} max={Math.ceil(total)} step={1} value={Math.min(moment, total)}
          aria-label={t('play')} onChange={(e) => setMoment(Number(e.target.value))} />
        {at && <p className="ui-muted sim-at-line">{t('at_line', { time: formatTime(at.time), distance: formatDistance(at.position, language), speed: kmh(at.speed) })}</p>}
        <div className="ui-row">
          <Button size="small" onClick={() => setMoment(m => Math.max(0, m - 30))}>−30 s</Button>
          <Button size="small" onClick={() => setMoment(m => Math.min(total, m + 30))}>+30 s</Button>
        </div>
      </Section>

      {result.warnings?.length > 0 && <Message tone="warn">{result.warnings.join(' ')}</Message>}
      <p className="ui-muted sim-credit">{t('credit')}</p>
    </Panel>
  )
}
