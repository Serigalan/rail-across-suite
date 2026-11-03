import { useDeferredValue, useEffect, useMemo, useState } from 'react'
import { loadTracks, loadSwitches, commitSwitchConnection, commitSwitchConnectionEdit } from '../../../storage'
import { elementStations } from '@rail-across/core/track'
import { SWITCH_TYPES, CONNECTION_SPEEDS, computeSwitchConnections, DEFAULT_MIDS, formPair } from '@rail-across/core/switch'
import { useI18n } from '../../../locales/i18nContext'
import {
  REASON_MSG, buildSCurve, computeShiftBounds, editCommit, hasStraightMid, preferredShift,
  settleConnection, shiftSnaps, stems, straightShifts, waOffset, SLIDER_REACH,
} from '@rail-across/core/commands'
import StationSlider from '../../form/StationSlider'
import { splitUnit } from '../../../locales/i18n'
import { TRACKS_HOVER_LAYER } from '../../../map/layerIds'
import usePreview from '../../../map/usePreview'
import ReadOnlyField from '../../form/ReadOnlyField'
import AdvancedInfo from '../../form/AdvancedInfo'
import CommitBar from '../../form/CommitBar'
import MidChainFields from './MidChainFields'
import CrossoverSolver from './CrossoverSolver'
import useConnectionPicks from './useConnectionPicks'
import PickPhase from './PickPhase'
import { PREVIEW_LAYERS, PREVIEW_SOURCE, POINTS_SOURCE, ldsCants, pointsGeoJSON, previewGeoJSON } from './connectionPreview'

const RAD2DEG = 180 / Math.PI
// Default speed — the one the R = 1200 form is built for, if it is in the table.
const DEFAULT_TYPE = Math.max(0,
  CONNECTION_SPEEDS.indexOf(SWITCH_TYPES.find(s => s.R === 1200)?.speed))

/**
 * The switch connection dialog. `edit` ({ found, apart, start } of
 * commands/crossoverEdit) opens it on a connection that exists (Entscheidung
 * 364): on the two tracks taken apart, the picks where the toes stood, its
 * middle elements as they are — the turnouts' form and the speed fixed. The
 * commit then replaces the old connection.
 */
export default function SCurveForm({ onCommitted, edit = null }) {
  const { t, fill } = useI18n()
  const { phase, picks, pickStatus, reset: resetPicks } = useConnectionPicks({ initial: edit?.apart.picks ?? null })
  const [speedIdx, setSpeedIdx] = useState(DEFAULT_TYPE)   // selected design speed (index into CONNECTION_SPEEDS)
  const [shiftRaw, setShift] = useState(edit?.start.shift ?? null)   // start-point offset along line 1 [m]; null: the preferred one
  // Straight the turnouts' through routes need beside the junction [m], set
  // when the commit found too little of it (see carveThrough).
  const [carveError, setCarveError] = useState(null)
  // The elements between the two branches (Entscheidung 305).
  const [mids, setMids] = useState(edit?.start.mids ?? DEFAULT_MIDS)
  // A chain is solved by iteration over the whole slider range: typing goes on
  // while it runs, and the solution follows the last value typed.
  const solvedMids = useDeferredValue(mids)
  // An answer of the Python solver adopted (Entscheidung 357): where its
  // iteration starts, and the shift it was adopted at — the slider's range is
  // the stretch around it.
  const [seed, setSeed] = useState(edit?.start.seed ?? null)
  const [adoptAt, setAdoptAt] = useState(null)

  // An edited connection keeps its form and its speed.
  const form     = edit?.found.form ?? null
  const speed    = edit ? edit.found.speed : CONNECTION_SPEEDS[speedIdx] ?? 0
  // The tracks the dialog works on: an edited connection's taken apart.
  const tracksNow = () => edit?.apart.tracks ?? loadTracks()

  const preview = usePreview(PREVIEW_LAYERS, { resetFilters: [TRACKS_HOVER_LAYER], resetCursor: true })

  // ── Where the slider may stand: both turnouts on the elements they were
  // picked on, the connection valid throughout — or, where it is valid
  // nowhere, refused by the rulebook alone (Entscheidung 361). It is stated as WA's station
  // from the begin of track 1 and snaps where WA or WE of either turnout fall
  // on an element change (Entscheidungen 309–310); untouched it stands where
  // WA lies on an element change (Entscheidung 170).
  const slider = useMemo(() => {
    if (phase !== 'config' || !picks[0] || !picks[1] || !speed) return null
    // Within SLIDER_REACH of the first click either way (Entscheidung 313):
    // shift 0 is where track 1 was clicked.
    // Where no valid connection stands, the stretch over which one only the
    // rulebook refuses does, to commit it anyway (Entscheidung 361).
    const valid = computeShiftBounds({ picks, speed, mids: solvedMids, seed, form, around: adoptAt })
    const lenient = !(valid.max > valid.min)
    const whole = lenient ? computeShiftBounds({ picks, speed, mids: solvedMids, seed, form, lenient }) : valid
    // Where the connection stands nowhere near the click, the whole range.
    // An adopted answer keeps its whole stretch: it may stand far from the click.
    const near = { min: Math.max(whole.min, -SLIDER_REACH), max: Math.min(whole.max, SLIDER_REACH) }
    const bounds = adoptAt == null && near.max >= near.min ? near : whole
    const track1 = tracksNow().find(tr => tr.id === picks[0].trackId)
    const elStart = track1 ? elementStations(track1)[picks[0].elIdx]?.start ?? 0 : 0
    const dir1 = stems(picks).g1.dir ?? 1
    return {
      bounds, lenient,
      snaps: shiftSnaps({ picks, speed, mids: solvedMids, seed, form, lenient }, bounds),
      preferred: preferredShift({ picks, speed, mids: solvedMids, seed, form, lenient }, bounds),
      // Where the element between the branches is a straight, the connection one wants.
      straights: straightShifts({ picks, speed, mids: solvedMids, seed, form, lenient }, bounds),
      offset: waOffset(picks),
      toStation: (sh) => elStart + picks[0].along + dir1 * sh,
      toShift: (st) => (st - elStart - picks[0].along) * dir1,
    }
    // tracksNow follows `edit`, which is fixed for the dialog's life
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [phase, picks, speed, solvedMids, seed, adoptAt, form])

  // The shift asked for, held inside the range the connection stands over.
  const shift = slider
    ? Math.min(slider.bounds.max, Math.max(slider.bounds.min, shiftRaw ?? slider.preferred))
    : 0

  // Which speeds these two tracks can be connected with where the slider stands
  // — the dropdown's disabled state. It follows from the picks and the shift, so
  // it is derived rather than pushed into state by an effect.
  const connections = useMemo(() => {
    if (!picks[0] || !picks[1]) return []
    const { g1, g2 } = stems(picks)
    return computeSwitchConnections(g1, g2, shift, solvedMids, seed)
  }, [picks, shift, solvedMids, seed])

  // ── The connection itself ─────────────────────────────────────────────────
  // Closed-form and pure, so it is derived from the picks, the speed and the
  // slider rather than pushed into state by an effect. A turnout that would
  // leave too short a piece of its element before WA is pushed onto the
  // element's node (settleConnection, LP.EL.01).
  const settled = useMemo(() => (
    phase === 'config' && picks[0] && picks[1] && speed
      ? settleConnection({ picks, speed, shift, mids: solvedMids, seed, form })
      : null
  ), [phase, picks, speed, shift, solvedMids, seed, form])
  const result = settled?.result ?? null
  // A connection to show and build: valid, or refused by the rulebook alone.
  const stands = !!(result?.valid || result?.overridable)

  // ── Preview (the map is the only thing outside React here) ────────────────
  useEffect(() => {
    if (stands) {
      preview.set(PREVIEW_SOURCE, previewGeoJSON(result))
      preview.set(POINTS_SOURCE, pointsGeoJSON(result))
    } else {
      preview.clear()
    }
  }, [result, stands, preview])

  const handleSpeedChange = (i) => {
    setSpeedIdx(i)
    setSeed(null)
    setAdoptAt(null)
  }
  // The middle elements as the user types them: an adopted answer's seed and
  // stretch no longer apply.
  const changeMids = (next) => {
    setMids(next)
    setSeed(null)
    setAdoptAt(null)
  }
  const adoptSolution = ({ shift: s, mids: m, seed: sd }) => {
    setCarveError(null)
    setMids(m)
    setSeed(sd)
    setAdoptAt(s)
    setShift(s)
  }

  const reset = () => {
    preview.clear()
    resetPicks()
    setShift(null)
    setCarveError(null)
    setMids(DEFAULT_MIDS)
    setSeed(null)
    setAdoptAt(null)
  }
  const handleCancel = () => { reset(); onCommitted?.() }

  // `force`: built although the rulebook refuses it (Entscheidung 361).
  const handleCommit = (force = false) => {
    const commit = edit
      ? buildSCurve({
        result, picks, tracks: edit.apart.tracks, switches: edit.apart.switches, speed, force,
        identities: edit.found.turnouts.map(tu => tu.sw), connTrack: edit.found.conn,
      })
      : buildSCurve({ result, picks, tracks: loadTracks(), switches: loadSwitches(), speed, force })
    if (!commit) return
    if (commit.carveError) { setCarveError(commit.carveError); return }
    if (edit) commitSwitchConnectionEdit(editCommit(edit.found, edit.apart, commit))
    else commitSwitchConnection(commit)
    reset()
    onCommitted?.({ switchIds: commit.addSwitches.map(sw => sw.switchId) })
  }

  // ── Render ─────────────────────────────────────────────────────────────────
  if (phase === 'config') {
    const deg = (r) => (r * RAD2DEG).toFixed(3)
    // Where a joint of the middle elements lies: at the start of the element
    // it comes before, the last at the end of the last one.
    const midCount = result?.midPieces?.length ?? 1
    const jointText = (k) => (k < midCount
      ? fill('scurve_joint_start', { n: String(k + 1) })
      : fill('scurve_joint_end', { n: String(midCount) }))
    // WA's distance from the element change behind it, for the slider's label.
    const wa = (s) => {
      const d = s + (slider?.offset ?? 0)
      return Math.abs(d - Math.round(d)) < 5e-3 ? String(Math.round(d)) : d.toFixed(2)
    }
    return (
      <>
        <div className="element-form">
          {edit && (
            <ReadOnlyField label={t('crossover_edit_form')}
              value={`${[...new Set(formPair(form).map(f => f.label))].join(' / ')} · ${speed} km/h`} />
          )}
          {!edit && <div className="form-field">
            <label>{t('field_speed')}</label>
            <select value={speedIdx} onChange={e => { setCarveError(null); handleSpeedChange(Number(e.target.value)) }}>
              {/* A speed without a connection where the slider stands stays
                  selectable: the reason shows, and the Python solver may still
                  find one (Entscheidung 357). */}
              {CONNECTION_SPEEDS.map((sp, i) => (
                <option key={sp} value={i}>
                  {connections[i] && !connections[i].valid ? fill('scurve_speed_none_here', { v: String(sp) }) : `${sp} km/h`}
                </option>
              ))}
            </select>
          </div>}

          {slider && (() => {
            const ends = [slider.toStation(slider.bounds.min), slider.toStation(slider.bounds.max)].sort((a, b) => a - b)
            const moved = settled && Math.abs(settled.shift - shift) > 1e-6
            return (
              <StationSlider label={splitUnit(t('scurve_station')).text}
                value={String(Math.round(slider.toStation(shift) * 1e4) / 1e4)}
                onChange={text => {
                  const st = Number(text)
                  if (text === '' || !Number.isFinite(st)) return
                  setCarveError(null)
                  setShift(slider.toShift(st))
                }}
                intervals={ends[1] > ends[0] ? [ends] : []}
                snaps={slider.snaps.map(slider.toStation)}
                hint={fill('scurve_wa_after_joint', { d: moved ? `${wa(shift)} → ${wa(settled.shift)}` : wa(shift) })} />
            )
          })()}

          <MidChainFields mids={mids} onChange={(next) => { setCarveError(null); changeMids(next) }}
            solved={result?.midPieces?.map(p => p.route)
              ?? (stands ? [{ length: result.Lg, r1: result.signedRg, r2: result.signedRg }] : null)} />

          <AdvancedInfo>
            <ReadOnlyField label={t('scurve_line1')} value={picks[0]?.name ?? ''} />
            <ReadOnlyField label={t('scurve_line2')} value={picks[1]?.name ?? ''} />
            <ReadOnlyField label={t('scurve_gap')} value={result ? `${result.gap.toFixed(2)} m` : ''} />
            {stands && (<>
            <ReadOnlyField label={t('scurve_switch_type')} value={result.switchType.label} />
            <ReadOnlyField label={t('scurve_angle')} value={`1:${result.switchType.ratio}  (${deg(result.w)}°)`} />
            <ReadOnlyField label={t('scurve_delta')} value={`${deg(result.delta)}°`} />
            <ReadOnlyField label={t('scurve_zgl')} value={`${result.Lg.toFixed(2)} m`} />
            {!result.midPieces && (
              <ReadOnlyField label={t('scurve_mid_radius')} value={result.signedRg
                ? `${Math.abs(result.signedRg).toFixed(0)} m`
                : t('scurve_mid_straight')} />
            )}
            {/* A branch bent straight is built as the plain form with its
                routes swapped: the running track is then its branch, on the form's radius. */}
            <ReadOnlyField label={t('scurve_branch_radius')} value={[result.signedR1, result.signedR2]
                .map(r => (r ? `${Math.abs(r).toFixed(0)} m`
                  : fill('scurve_branch_swapped', { r: String(result.switchType.R) })))
                .join('  /  ')} />
            {result.cantMid !== 0 && (
              <ReadOnlyField label={t('scurve_cant')} value={`${Math.abs(result.cantMid)} mm`} />
            )}
            <ReadOnlyField label={t('scurve_total')} value={`${result.laenge.toFixed(2)} m`} />
            </>)}
          </AdvancedInfo>
        </div>

        <p className={result?.valid ? 'msg-info' : 'msg-error'}>
          {result?.valid ? t('scurve_valid')
            : result?.reason === 'cant_mismatch' ? fill('scurve_cant_mismatch', ldsCants(result))
              : result?.reason === 'mid_element_short' && result.midShort ? fill('scurve_mid_element_short_at', {
                n: String(result.midShort.index + 1), l: result.midShort.length.toFixed(2),
                lmin: result.midShort.lMin.toFixed(2), v: String(speed),
              })
              : result?.reason === 'mid_jump' && result.midJump ? fill('scurve_mid_jump_at', {
                at: jointText(result.midJump.index), rw: result.midJump.rw.toFixed(0),
                limit: result.midJump.limit.toFixed(0), v: String(speed),
              })
              : t(REASON_MSG[result?.reason] ?? 'scurve_invalid')}
        </p>
        {result?.valid && result.midJumpHints?.map(h => (
          <p key={h.index} className="msg-hint">
            {fill('scurve_mid_jump_hint', { at: jointText(h.index), rw: h.rw.toFixed(0), reg: h.reg.toFixed(0) })}
          </p>
        ))}
        {slider && !result?.valid && slider.lenient && (
          <CrossoverSolver picks={picks} speed={speed} form={form} stationOf={slider.toStation} onAdopt={adoptSolution} />
        )}
        {stands && !hasStraightMid(result) && slider?.straights.length > 0 && (() => {
          const near = [...slider.straights].sort((a, b) => Math.abs(a - shift) - Math.abs(b - shift))[0]
          return (
            <div className="msg-hint">
              <p>{fill('scurve_straight_possible', { st: slider.toStation(near).toFixed(2) })}</p>
              <button type="button" className="ui-btn ui-btn-block" onClick={() => { setCarveError(null); setShift(near) }}>
                {t('scurve_straight_go')}
              </button>
            </div>
          )
        })()}
        {settled?.moved.length > 0 && (
          <p className="msg-hint">
            {fill('scurve_min_element_moved', { tracks: settled.moved.map(k => picks[k]?.name ?? '').join(', ') })}
          </p>
        )}
        {result?.valid && settled.remnants.some(r => r.lMin == null) && (
          <p className="msg-hint">
            {fill('scurve_min_element_unchecked', {
              tracks: [...new Set(settled.remnants.filter(r => r.lMin == null).map(r => picks[r.track]?.name ?? ''))].join(', '),
            })}
          </p>
        )}
        {result?.short && (
          <p className="msg-error">
            {fill('scurve_min_element_short', {
              side: t(result.short.side === 'before' ? 'scurve_side_before_wa' : 'scurve_side_after_we'),
              track: picks[result.short.track]?.name ?? '',
              l: result.short.run.toFixed(2),
              lmin: result.short.lMin.toFixed(2),
              v: String(result.short.v ?? ''),
            })}
          </p>
        )}
        {carveError != null && (
          <p className="form-error">
            {fill('switch_on_track_no_room', { m: carveError.toFixed(1) })}
          </p>
        )}

        <CommitBar onCommit={handleCommit} onCancel={handleCancel} disabled={!result?.valid}
          onOverride={result?.overridable ? () => handleCommit(true) : null} />
      </>
    )
  }

  // select_first / select_second
  return <PickPhase phase={phase} picks={picks} pickStatus={pickStatus} onCancel={handleCancel} />
}
