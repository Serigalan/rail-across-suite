import { useState } from 'react'
import maplibregl from 'maplibre-gl'
import { crsName, planeCoordsToWgs84 } from '@rail-across/core/geometry'
import { heightDatumLabel } from '@rail-across/core/track'
import { count, duration, mb, sizeText } from '../../../utils/pointCloud'
import { useI18n } from '../../../locales/i18nContext'
import { formatDate } from '../../../locales/i18n'
import { useMap } from '../../../map/MapContext'
import ConfirmModal from '../../ConfirmModal'
import FilePickButton from '../../form/FilePickButton'

const crsText = (code) => crsName(code) ?? `EPSG ${code}`

/** How far a loading of this tab has come, and the button that stops it. */
function UploadProgress({ upload }) {
  const { t, fill } = useI18n()
  const p = upload.progress
  const share = p.total ? p.sent / p.total : 0
  return (
    <div className="pointcloud-progress">
      <progress max={1} value={share} />
      <span className="pointcloud-meta">
        {fill('pointcloud_uploading', { share: Math.floor(share * 100), sent: mb(p.sent), total: mb(p.total) })
          + (p.rate ? ` · ${mb(p.rate)}/s · ${t('pointcloud_remaining')} ${duration(p.remaining)}` : '')}
      </span>
      <button className="ui-btn" onClick={upload.abort}>{t('pointcloud_upload_abort')}</button>
    </div>
  )
}

/** What a cloud on the server is doing, and what can be done about it. */
function ServerStatus({ cloud, upload, mayEdit, onResume, onRetry }) {
  const { t, fill } = useI18n()
  if (cloud.status === 'uploading') {
    if (upload) return <UploadProgress upload={upload} />
    const share = Math.floor(100 * cloud.received / cloud.file.size)
    return (
      <>
        <span className="pointcloud-meta">{fill('pointcloud_upload_stopped', { share, file: cloud.file.name })}</span>
        {mayEdit && (
          <FilePickButton accept=".laz,.las,.e57" className="ui-btn" onFile={(file) => onResume(cloud, file)}>
            {t('pointcloud_upload_resume')}
          </FilePickButton>
        )}
      </>
    )
  }
  if (cloud.status === 'queued') return <span className="pointcloud-meta">{t('pointcloud_status_queued')}</span>
  if (cloud.status === 'processing') {
    return (
      <div className="pointcloud-progress">
        <progress max={1} value={cloud.progress ?? 0} />
        <span className="pointcloud-meta">{fill('pointcloud_status_processing', { share: Math.floor(100 * (cloud.progress ?? 0)) })}</span>
      </div>
    )
  }
  if (cloud.status === 'failed') {
    return (
      <>
        <span className="form-error">{fill('pointcloud_status_failed', { reason: cloud.error ?? '' })}</span>
        {mayEdit && <button className="ui-btn" onClick={() => onRetry(cloud)}>{t('pointcloud_retry')}</button>}
      </>
    )
  }
  return null
}

/** Fit the map to a box in a plane. */
function fitPlaneBox(map, bounds, crs) {
  const corners = planeCoordsToWgs84([[bounds.minE, bounds.minN], [bounds.maxE, bounds.maxN]], crs)
  map.fitBounds(new maplibregl.LngLatBounds(corners[0], corners[0]).extend(corners[1]), { padding: 80, maxZoom: 19 })
}

/**
 * The project's clouds (AP 13.6), on the project server and seen by every
 * member, with the state of their loading or preparation: where each lies,
 * how big, and, for whoever may, delete, resume a loading or start a failed
 * preparation again. `clouds` is null until the server has answered.
 */
export default function CloudList({ clouds, uploads, mayEdit, onDelete, onResume, onRetry, onOpen3d }) {
  const { t, fill, language } = useI18n()
  const map = useMap()
  const [asking, setAsking] = useState(null)    // the cloud a delete waits on

  return (
    <>
      <h3 className="pointcloud-list-title">{t('pointcloud_list')}</h3>
      {clouds == null && <p className="selecting-hint">…</p>}
      {clouds?.length === 0 && <p className="selecting-hint">{t('pointcloud_none')}</p>}
      {(clouds ?? []).map(c => (
        <div key={c.id} className="pointcloud-item">
          <span><strong>{c.name}</strong></span>
          <span className="pointcloud-meta">
            {`${t('pointcloud_crs_short')}: ${c.crs == null ? t('pointcloud_crs_local') : crsText(c.crs)}`
              + ` · ${t('pointcloud_height_short')}: ${heightDatumLabel(c.heightEpsg)}`
              + (c.rgb ? ` · ${t('pointcloud_rgb')}` : '')}
          </span>
          {c.status === 'ready' && (
            <span className="pointcloud-meta">
              {`${fill('pointcloud_points', { n: count(c.levels[0]?.points ?? c.sourcePoints) })} · `
                + `${fill('pointcloud_levels', { n: c.levels.length })} · ${sizeText(c.bytes)}`}
            </span>
          )}
          <span className="pointcloud-meta">{`${c.file.name} · ${sizeText(c.file.size)} · ${formatDate(c.createdAt, language, { time: true })}`}</span>
          <ServerStatus cloud={c} upload={uploads.find(u => u.cloudId === c.id)} mayEdit={mayEdit} onResume={onResume} onRetry={onRetry} />
          <div className="pointcloud-actions">
            {c.status === 'ready' && c.crs != null && c.bounds && (
              <button className="ui-btn ui-btn-small" onClick={() => map?.current && fitPlaneBox(map.current, c.bounds, c.crs)}>
                {t('pointcloud_show')}
              </button>
            )}
            {c.status === 'ready' && c.crs != null && onOpen3d && (
              <button className="ui-btn ui-btn-small" title={t('pointcloud_open_3d_hint')} onClick={() => onOpen3d(c)}>
                {t('pointcloud_open_3d')}
              </button>
            )}
            {mayEdit && !uploads.some(u => u.cloudId === c.id) && (
              <button className="ui-btn ui-btn-small ui-btn-danger" onClick={() => setAsking(c)}>{t('modal_delete')}</button>
            )}
          </div>
        </div>
      ))}
      {asking && (
        <ConfirmModal
          message={fill('pointcloud_delete_ask_server', { name: asking.name })}
          onConfirm={() => { onDelete(asking); setAsking(null) }}
          onCancel={() => setAsking(null)} />
      )}
    </>
  )
}
