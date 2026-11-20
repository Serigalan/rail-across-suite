import { useMemo, useState } from 'react'
import {
  MIN_RANGE, SPEED_STEPS, SVG_SCALE, averageSpeed, runOf, speedProfile, speedRanges, speedRangesCsv, speedRangesSvg, speedRangesSvgSize, withKm,
} from '../../utils/exchange'
import { downloadText } from '../../utils/common'
import { useI18n } from '../../locales/i18nContext'
import { useProject } from '../../hooks/useStore'
import NumberInput from '../form/NumberInput'
import Modal from '../Modal'

const byName = (a, b) => String(a.name ?? '').localeCompare(String(b.name ?? ''), 'de', { numeric: true })
const km3 = (m) => (m == null ? '–' : (m / 1000).toFixed(3).replace('.', ','))

/**
 * The export behind the speed band (Trassenbänder): the highest speeds along
 * a run — the band's track, or a named route or another track —, from the
 * design speed of its elements, in ranges no shorter than the length given
 * (800 m unless another is), in steps of 5 or 10 km/h, laid for the highest
 * average speed; listed here, the diagram drawn to a length and a height scale
 * with their kilometres, and given out as a CSV table or an SVG diagram.
 */
export default function SpeedExportDialog({ trackId, onClose }) {
  const { t, fill } = useI18n()
  const project = useProject()
  const routes = [...(project?.routes ?? [])].sort(byName)
  const tracks = [...(project?.tracks ?? [])].sort(byName)
  const [chosen, setChosen] = useState(trackId ? `track:${trackId}` : '')
  const [min, setMin] = useState(String(MIN_RANGE))
  const [speedStep, setSpeedStep] = useState(SPEED_STEPS[0])
  // The diagram to scale: how many millimetres a kilometre is long and a km/h high.
  const [mmPerKm, setMmPerKm] = useState(String(SVG_SCALE.mmPerKm))
  const [mmPerKmh, setMmPerKmh] = useState(String(SVG_SCALE.mmPerKmh))
  const scaleOf = () => ({ mmPerKm: Number(mmPerKm), mmPerKmh: Number(mmPerKmh) })
  const size = ({ W, H }) => ({ w: Math.round(W), h: Math.round(H) })
  const choice = chosen || (routes[0] ? `route:${routes[0].id}` : tracks[0] ? `track:${tracks[0].id}` : '')

  const result = useMemo(() => {
    if (!project || !choice) return null
    const [kind, id] = choice.split(/:(.*)/s)
    const trackIds = kind === 'route' ? routes.find(r => r.id === id)?.trackIds ?? [] : [id]
    const run = runOf(trackIds, project.tracks, project.switches ?? [])
    if (!run.parts?.length) return null
    const profile = speedProfile(run)
    const ranges = withKm(run, speedRanges(profile, { min: Math.max(0, Number(min) || 0), speedStep }), project.kmLines)
    const name = kind === 'route' ? routes.find(r => r.id === id)?.name : tracks.find(tr => tr.id === id)?.name
    return { run, profile, ranges, name: name ?? '', average: averageSpeed(ranges), unknown: profile.filter(p => p.speed == null).reduce((s, p) => s + p.to - p.from, 0) }
  }, [project, choice, min, speedStep, routes, tracks])

  const columns = { kmFrom: t('speed_ranges_km_from'), kmTo: t('speed_ranges_km_to'), from: t('speed_ranges_from'), to: t('speed_ranges_to'), length: t('speed_ranges_length'), speed: t('speed_ranges_speed') }
  const fileName = (ext) => `${project.title}_${result.name}_geschwindigkeiten.${ext}`.replace(/[\\/:*?"<>|]/g, '_')
  const svg = () => speedRangesSvg(result.profile, result.ranges, {
    title: fill('speed_ranges_svg_title', { name: result.name }),
    subtitle: fill('speed_ranges_svg_subtitle', { min: Number(min) || 0, step: speedStep, average: result.average != null ? result.average.toFixed(1).replace('.', ',') : '–' }),
    speedAxis: t('speed_ranges_speed'), legendDesign: t('speed_ranges_legend_design'), legendRanges: t('speed_ranges_legend_ranges'),
  }, scaleOf())
  const drawable = result?.ranges.length > 0 && Number(mmPerKm) > 0 && Number(mmPerKmh) > 0

  return (
    <Modal title={t('speed_ranges_title')} onClose={onClose} className="band-export-dialog"
      actions={<>
        <button type="button" className="ui-btn" onClick={onClose}>{t('btn_close')}</button>
        <button type="button" className="ui-btn" disabled={!result}
          onClick={() => downloadText(speedRangesCsv(result.ranges, columns), fileName('csv'), 'text/csv')}>
          {t('speed_ranges_csv')}
        </button>
        <button type="button" className="ui-btn ui-btn-primary" disabled={!drawable}
          onClick={() => downloadText(svg(), fileName('svg'), 'image/svg+xml')}>
          {t('speed_ranges_svg')}
        </button>
      </>}>
      <p className="modal-message">{t('speed_ranges_desc')}</p>
      <div className="form-field">
        <label>{t('speed_ranges_run')}</label>
        <select className="settings-select" value={choice} onChange={e => setChosen(e.target.value)} disabled={!project}>
          {routes.length > 0 && (
            <optgroup label={t('speed_ranges_routes')}>
              {routes.map(r => <option key={r.id} value={`route:${r.id}`}>{r.name ?? r.id}</option>)}
            </optgroup>
          )}
          <optgroup label={t('speed_ranges_tracks')}>
            {tracks.map(tr => <option key={tr.id} value={`track:${tr.id}`}>{tr.name ?? tr.id}</option>)}
          </optgroup>
        </select>
      </div>
      <div className="form-field">
        <label>{t('speed_ranges_step')}</label>
        <select className="settings-select" value={speedStep} onChange={e => setSpeedStep(Number(e.target.value))}>
          {SPEED_STEPS.map(v => <option key={v} value={v}>{fill('speed_ranges_step_option', { v })}</option>)}
        </select>
      </div>
      <div className="form-field">
        <label>{t('speed_ranges_min')}</label>
        <NumberInput min="0" step={50} value={min} onChange={e => setMin(e.target.value)} unit="m" />
      </div>
      {result && <>
        {!result.run.ok && <p className="msg-hint">{t('speed_ranges_gaps')}</p>}
        <p>{fill('speed_ranges_average', { average: result.average != null ? result.average.toFixed(1).replace('.', ',') : '–', n: result.ranges.length })}</p>
        {result.unknown > 0.5 && <p className="msg-hint">{fill('speed_ranges_unknown', { m: Math.round(result.unknown) })}</p>}
        <div className="band-export-table">
          <table className="speed-ranges-table">
            <thead><tr><th>{columns.kmFrom}</th><th>{columns.kmTo}</th><th>{columns.length}</th><th>{columns.speed}</th></tr></thead>
            <tbody>
              {result.ranges.map(r => (
                <tr key={r.from}>
                  <td>{r.kmFrom != null ? km3(r.kmFrom) : `${Math.round(r.from)} m`}</td>
                  <td>{r.kmTo != null ? km3(r.kmTo) : `${Math.round(r.to)} m`}</td>
                  <td>{Math.round(r.to - r.from)} m</td>
                  <td>{r.speed ?? '–'}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <div className="form-field mt-2">
          <label>{t('speed_ranges_scale_length')}</label>
          <NumberInput min="1" step={10} value={mmPerKm} onChange={e => setMmPerKm(e.target.value)} unit="mm" />
        </div>
        {Number(mmPerKm) > 0 && <p className="msg-hint">{fill('speed_ranges_scale_length_is', { n: Math.round(1e6 / Number(mmPerKm)).toLocaleString('de') })}</p>}
        <div className="form-field">
          <label>{t('speed_ranges_scale_height')}</label>
          <NumberInput min="0.1" step={1} value={mmPerKmh} onChange={e => setMmPerKmh(e.target.value)} unit="mm" />
        </div>
        {drawable && <p className="msg-hint">{fill('speed_ranges_svg_size', size(speedRangesSvgSize(result.profile, result.ranges, scaleOf())))}</p>}
      </>}
    </Modal>
  )
}
