import { useState } from 'react'
import { BAND_SVG_MM_PER_KM, BAND_SVG_MM_PER_UNIT, bandSvg, bandSvgSize } from '../../utils/exchange'
import { downloadText } from '../../utils/common'
import { useI18n } from '../../locales/i18nContext'
import { useProject } from '../../hooks/useStore'
import NumberInput from '../form/NumberInput'
import Modal from '../Modal'

/**
 * The export behind a band of the Trassenbänder (curvature, cant, height
 * plan): the band as an SVG drawing to scale — a length scale, how many
 * millimetres a kilometre of the track is, and a height scale, how many one
 * unit of the band is —, over the whole track or the stretch the overlay
 * shows (`view`, [from, to] m). `band` names it, `spec` is what bandSpecs
 * gives for it, `boundaries` the stations of the element joints.
 */
export default function BandExportDialog({ band, spec, track, length, view, boundaries, onClose }) {
  const { t, fill } = useI18n()
  const project = useProject()
  const [stretch, setStretch] = useState('track')
  const [mmPerKm, setMmPerKm] = useState(String(BAND_SVG_MM_PER_KM))
  const [mmPerUnit, setMmPerUnit] = useState(String(BAND_SVG_MM_PER_UNIT[band]))
  const zoomed = view && (view[0] > 0.5 || view[1] < length - 0.5)
  const range = stretch === 'view' && zoomed ? { from: Math.max(0, view[0]), to: Math.min(length, view[1]) } : { from: 0, to: length }
  const scale = { mmPerKm: Number(mmPerKm), mmPerUnit: Number(mmPerUnit) }
  const ok = length > 0 && range.to > range.from && scale.mmPerKm > 0 && scale.mmPerUnit > 0
  const size = ok ? bandSvgSize(spec, range, scale) : null
  const name = track.name || track.id.slice(0, 8)
  const m = (s) => Math.round(s).toLocaleString('de')

  const download = () => {
    const svg = bandSvg(spec, {
      boundaries, range, scale,
      texts: {
        title: spec.title,
        subtitle: fill('bands_export_subtitle', { track: name, from: m(range.from), to: m(range.to) }),
        note: spec.rightUp ? t('bands_right_up') : '',
        station: t('elevation_station'),
        empty: spec.empty || '',
      },
    })
    const file = `${project?.title ?? ''}_${name}_${spec.title}.svg`.replace(/[\\/:*?"<>|]/g, '_')
    downloadText(svg, file, 'image/svg+xml')
  }

  return (
    <Modal title={fill('bands_export_title', { band: spec.title })} onClose={onClose} className="band-export-dialog"
      actions={<>
        <button type="button" className="ui-btn" onClick={onClose}>{t('btn_cancel')}</button>
        <button type="button" className="ui-btn ui-btn-primary" disabled={!ok} onClick={download}>{t('bands_export_svg')}</button>
      </>}>
      <p className="modal-message">{fill('bands_export_desc', { track: name })}</p>
      <div className="form-field">
        <label>{t('bands_export_stretch')}</label>
        <select className="settings-select" value={zoomed ? stretch : 'track'} onChange={e => setStretch(e.target.value)} disabled={!zoomed}>
          <option value="track">{fill('bands_export_stretch_track', { to: m(length) })}</option>
          {zoomed && <option value="view">{fill('bands_export_stretch_view', { from: m(view[0]), to: m(view[1]) })}</option>}
        </select>
      </div>
      <div className="form-field">
        <label>{t('speed_ranges_scale_length')}</label>
        <NumberInput min="1" step={10} value={mmPerKm} onChange={e => setMmPerKm(e.target.value)} unit="mm" />
      </div>
      {scale.mmPerKm > 0 && <p className="msg-hint">{fill('speed_ranges_scale_length_is', { n: Math.round(1e6 / scale.mmPerKm).toLocaleString('de') })}</p>}
      <div className="form-field">
        <label>{fill('bands_export_scale_height', { unit: spec.unit })}</label>
        <NumberInput min="0.01" step="any" value={mmPerUnit} onChange={e => setMmPerUnit(e.target.value)} unit="mm" />
      </div>
      {size && <p className="msg-hint">{fill('speed_ranges_svg_size', { w: Math.round(size.W), h: Math.round(size.H) })}</p>}
    </Modal>
  )
}
