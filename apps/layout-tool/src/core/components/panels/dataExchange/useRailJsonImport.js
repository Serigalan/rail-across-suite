import { useState } from 'react'
import { commitImport, loadEndMarks, loadTracks } from '../../../storage'
import { parseOsrdRailJson, planRailJsonImport } from '../../../utils/exchange'
import { fitToTracks } from '../../../map/mapRenderUtils'
import { readFileText } from '../../../utils/common'
import { fitImportedTracks } from '../../../utils/exchange'
import { useI18n } from '../../../locales/i18nContext'
import { useMap } from '../../../map/MapContext'

/**
 * Reading a RailJSON or alignment exchange file into the project (osrdImport):
 * the geometry comes from horizontal_alignment, the heights from
 * vertical_alignment. Tracks already present are not replaced — the imported
 * ones take a fresh id. What the app does not model is kept, so an export
 * hands it back. One commit, one undo step.
 */
export default function useRailJsonImport() {
  const { t } = useI18n()
  const map = useMap()
  const [errors, setErrors] = useState([])

  const importFile = async (file) => {
    setErrors([])
    try {
      const parsed = parseOsrdRailJson(JSON.parse(await readFileText(file)))
      setErrors(parsed.errors)
      if (!parsed.tracks.length) return
      const { osrd, ...commit } = planRailJsonImport(parsed, { existingTracks: loadTracks(), existingMarks: loadEndMarks() })
      // The tracks fitted to hold together before they are kept (decision 0006).
      const fitted = await fitImportedTracks(commit.addTracks)
      setErrors([...parsed.errors, ...fitted.notes])
      commitImport({ ...commit, addTracks: fitted.tracks, patch: { osrd } })
      // The import usually lands far outside the current view — show it.
      fitToTracks(map?.current, commit.addTracks)
    } catch (err) {
      setErrors([`${t('import_parse_error')}: ${err.message}`])
    }
  }

  return { errors, importFile }
}
