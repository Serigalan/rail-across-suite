import { useEffect, useState } from 'react'
import { outlineFeature, cloudServerHook } from '../../utils/pointCloud'
import { useI18n } from '../../locales/i18nContext'
import { useProject } from '../../hooks/useStore'
import { useMayEditClouds } from '../../hooks/useCurrentUser'
import usePreview from '../../map/usePreview'
import { PALETTE } from '../../styles/palette'
import CloudImportForm from './pointCloud/CloudImportForm'
import CloudList from './pointCloud/CloudList'
import CloudCacheSetting from './pointCloud/CloudCacheSetting'
import RailTraceSection from './pointCloud/RailTraceSection'
import AxisSurveyList from './pointCloud/AxisSurveyList'
import InfoTip from '../form/InfoTip'

// Where each cloud lies, while the panel is open.
const OUTLINE_SOURCE = 'pointcloud-outline-source'
const OUTLINE_LAYERS = [{
  sourceId: OUTLINE_SOURCE,
  layer: {
    id: 'pointcloud-outline-layer', type: 'line',
    paint: { 'line-color': PALETTE.cloudOutline, 'line-width': 1.5, 'line-dasharray': [2, 1.5] },
  },
}]

/**
 * Point clouds of the project (AP 13.6): they lie on the project server —
 * on the server, or in the desktop app the server in its own process — and
 * every member sees them. Loading one and deleting are for the admin and
 * whoever has the right (decision 203). Below the list finding a track's
 * axis in the clouds (AP 12.2) and the measured axes. The list, loading and
 * the 3D window are the server's (`useCloudServer`, server/useCloudServer.js).
 */
export default function PointCloudPanel() {
  const { t } = useI18n()
  const project = useProject()
  const mayEdit = useMayEditClouds()
  const [message, setMessage] = useState(null)  // { kind: 'error'|'done', text }
  // The measured axis whose points are being taken out on the map — the
  // trace's own clicks on the map wait meanwhile.
  const [erasingId, setErasingId] = useState(null)
  // A measured axis to trace anew after a re-referencing (AP 13.15).
  const [retrace, setRetrace] = useState(null)
  const useCloudServer = cloudServerHook()
  const remote = useCloudServer(project.id, { onMessage: setMessage })

  // The clouds the trace reads (level 1), and those whose outline is drawn (level 2).
  const readable = remote.readable
  const outline = usePreview(OUTLINE_LAYERS)
  useEffect(() => {
    outline.set(OUTLINE_SOURCE, { type: 'FeatureCollection', features: remote.outlines.map(outlineFeature) })
  }, [outline, remote.outlines])

  return (
    <>
      <h2>{t('pointcloud_title')}<InfoTip text={t('pointcloud_hint')} /></h2>
      {mayEdit && remote.upload
        ? <CloudImportForm onMessage={setMessage} onLoad={remote.upload} />
        : <p className="selecting-hint">{t('pointcloud_view_only')}</p>}
      {message && <p className={message.kind === 'error' ? 'form-error' : 'selecting-hint'}>{message.text}</p>}
      <CloudList clouds={remote.rows} uploads={remote.uploads} mayEdit={mayEdit}
        onDelete={remote.remove} onResume={remote.resume} onRetry={remote.retry} onOpen3d={remote.open3d} />
      {readable.length > 0 && (
        <RailTraceSection clouds={readable} paused={!!erasingId} retrace={retrace} onRetraced={() => setRetrace(null)} />
      )}
      <AxisSurveyList erasingId={erasingId} onErasing={setErasingId} cloudRows={remote.rows ?? []}
        onRetrace={readable.length ? setRetrace : null} />
      {(remote.rows ?? []).length > 0 && <CloudCacheSetting />}
    </>
  )
}
