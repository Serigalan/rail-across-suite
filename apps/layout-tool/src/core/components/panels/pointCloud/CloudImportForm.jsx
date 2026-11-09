import { useState } from 'react'
import { crsLabel } from '@rail-across/core/geometry'
import { HEIGHT_DATUMS, heightDatumLabel } from '@rail-across/core/track'
import {
  readFileHeader, probeExtent, previewPoints, formatName, pointsAsText, PREVIEW_POINTS, CLOUD_CRS,
  count, crsHint, mb,
} from '../../../utils/pointCloud'
import { downloadText } from '../../../utils/common'
import { useI18n } from '../../../locales/i18nContext'
import { useTracks } from '../../../hooks/useStore'
import FilePickButton from '../../form/FilePickButton'
import ReadOnlyField from '../../form/ReadOnlyField'
import CommitBar from '../../form/CommitBar'
import AdvancedInfo from '../../form/AdvancedInfo'
import Modal from '../../Modal'
import InfoTip from '../../form/InfoTip'

/** A file's name without its point cloud extension. */
const baseName = (name) => name.replace(/\.(laz|las|e57)$/i, '')

/** The first points of the file as text, to read the coordinates before choosing their system. */
function PointsTextModal({ preview, onClose }) {
  const { t, fill } = useI18n()
  const [copied, setCopied] = useState(false)
  const copy = async () => {
    try { await navigator.clipboard.writeText(preview.text); setCopied(true) } catch { setCopied(false) }
  }
  return (
    <Modal className="pointcloud-text-modal" title={fill('pointcloud_text_title', { n: PREVIEW_POINTS })} onClose={onClose}
      actions={<>
        <button type="button" className="ui-btn" onClick={copy}>
          {copied ? t('pointcloud_text_copied') : t('pointcloud_text_copy')}
        </button>
        <button type="button" className="ui-btn"
          onClick={() => downloadText(preview.text, `${baseName(preview.name)}_${PREVIEW_POINTS}.txt`)}>
          {t('pointcloud_text_save')}
        </button>
        <button type="button" className="ui-btn ui-btn-primary" onClick={onClose}>{t('pointcloud_text_close')}</button>
      </>}>
      <pre className="pointcloud-text">{preview.text}</pre>
    </Modal>
  )
}

/**
 * A LAS, LAZ or E57 file, loaded onto the project server (AP 13.2) — on the
 * server, or in the desktop app the server in its own process; the server
 * makes every level of it in the file's plane. It asks for the horizontal and
 * the vertical datum of the file before anything is sent — both required,
 * neither preselected (Entscheidung 118), "local / unknown" allowed (decision
 * 214) — and checks the box the file states for itself against the tracks, so
 * a wrong choice shows before an hour of preparing. `onLoad` gets what loading
 * needs and runs it (useCloudServer's upload); `onMessage` reports what went
 * wrong reading the file.
 */
export default function CloudImportForm({ onMessage, onLoad }) {
  const { t, fill } = useI18n()
  const tracks = useTracks()
  const [pick, setPick] = useState(null)        // { file, header } once a file is chosen
  const [crs, setCrs] = useState('')
  const [heightEpsg, setHeightEpsg] = useState('')
  const [keepRaw, setKeepRaw] = useState(false)
  const [preview, setPreview] = useState(null)  // { busy } while reading, then { text, name }

  const choose = async (file) => {
    onMessage(null)
    try {
      setPick({ file, header: await readFileHeader(file) })
      setCrs('')
      setHeightEpsg('')
      setKeepRaw(false)
    } catch (err) {
      setPick(null)
      onMessage({ kind: 'error', text: `${t('pointcloud_not_las')} (${err.message})` })
    }
  }

  const showPreview = async () => {
    setPreview({ busy: true })
    try {
      const { header, points } = await previewPoints(pick.file, PREVIEW_POINTS)
      const labels = {
        format: t('pointcloud_text_format'), points: t('pointcloud_text_points'),
        scale: t('pointcloud_text_scale'), offset: t('pointcloud_text_offset'),
        first: t('pointcloud_text_first'), intensity: t('pointcloud_text_intensity'),
        scans: t('pointcloud_text_scans'), crs: t('pointcloud_text_crs'),
      }
      setPreview({ name: pick.file.name, text: pointsAsText(header, points, { name: pick.file.name, labels }) })
    } catch (err) {
      setPreview(null)
      onMessage({ kind: 'error', text: `${t('pointcloud_text_failed')}: ${err.message}` })
    }
  }

  const localCrs = crs === 'local'

  const begin = () => {
    if (!pick || !crs || !heightEpsg) return
    const { file, header } = pick
    setPick(null)
    onLoad({ file, header, name: baseName(file.name), crs: localCrs ? null : Number(crs), heightEpsg: Number(heightEpsg), keepRaw })
  }

  if (!pick) {
    return (
      <div className="pointcloud-actions">
        <FilePickButton accept=".laz,.las,.e57" onFile={choose} className="ui-btn ui-btn-primary" title={t('pointcloud_upload_hint')}>
          {t('pointcloud_upload')}
        </FilePickButton>
      </div>
    )
  }

  const probe = crs && !localCrs ? probeExtent(pick.header, Number(crs), tracks) : null
  return (
    <div className="pointcloud-import">
      <AdvancedInfo>
        <ReadOnlyField label={t('pointcloud_file')} value={`${pick.file.name} · ${mb(pick.file.size)}`} />
      </AdvancedInfo>
      <span className="pointcloud-meta">
        {`${formatName(pick.header)} · `
          + (pick.header.format === 'e57' ? `${fill('pointcloud_scans', { n: count(pick.header.scans.length) })} · ` : '')
          + `${fill('pointcloud_points', { n: count(pick.header.pointCount) })}`
          + (pick.header.rgb ? ` · ${t('pointcloud_rgb')}` : '')}
      </span>
      {pick.header.coordinateMetadata && (
        <span className="pointcloud-meta">{fill('pointcloud_file_crs', { crs: crsHint(pick.header.coordinateMetadata) })}</span>
      )}
      <button className="ui-btn pointcloud-text-btn" disabled={preview?.busy} onClick={showPreview}>
        {preview?.busy ? t('pointcloud_text_reading') : fill('pointcloud_text_show', { n: PREVIEW_POINTS })}
      </button>
      <div className="form-field">
        <label>{t('pointcloud_crs')}{localCrs && <InfoTip text={t('pointcloud_crs_local_hint')} />}</label>
        <select value={crs} onChange={e => setCrs(e.target.value)}>
          <option value="">{t('pointcloud_choose')}</option>
          {CLOUD_CRS.map(code => <option key={code} value={code}>{crsLabel(code)}</option>)}
          <option value="local">{t('pointcloud_crs_local')}</option>
        </select>
      </div>
      <div className="form-field">
        <label>{t('pointcloud_height')}</label>
        <select value={heightEpsg} onChange={e => setHeightEpsg(e.target.value)}>
          <option value="">{t('pointcloud_choose')}</option>
          {HEIGHT_DATUMS.map(d => <option key={d.epsg} value={d.epsg}>{heightDatumLabel(d.epsg)}</option>)}
        </select>
      </div>
      <label className="transition-curve-row">
        <input type="checkbox" checked={keepRaw} onChange={e => setKeepRaw(e.target.checked)} />
        <span>{t('pointcloud_keep_raw')}</span>
        <InfoTip text={<><p>{t('pointcloud_upload_levels')}</p><p>{t('pointcloud_keep_raw_hint')}</p></>} />
      </label>
      {probe?.ok === true && <p className="selecting-hint">{t('pointcloud_probe_ok')}</p>}
      {probe?.ok === false && <p className="form-error">{t('pointcloud_probe_off')}</p>}
      {probe && probe.ok === null && <p className="selecting-hint">{t('pointcloud_probe_no_tracks')}</p>}
      <CommitBar onCommit={begin} onCancel={() => setPick(null)} disabled={!crs || !heightEpsg}
        commitLabel={t('pointcloud_upload_start')} className="" />
      {preview?.text && <PointsTextModal preview={preview} onClose={() => setPreview(null)} />}
    </div>
  )
}
