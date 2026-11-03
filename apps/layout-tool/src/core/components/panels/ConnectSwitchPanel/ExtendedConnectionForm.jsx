import { useEffect, useMemo, useRef, useState } from 'react'
import { loadTracks, loadSwitches, commitSwitchConnection, commitSwitchConnectionEdit } from '../../../storage'
import { elementStations } from '@rail-across/core/track'
import { bauform, SWITCH_PICK_TYPES, SWITCH_TYPES } from '@rail-across/core/switch'
import {
  REASON_MSG, adoptCrossover, buildSCurve, crossoverRequest, editCommit, settleConnection, stems,
} from '@rail-across/core/commands'
import { useI18n } from '../../../locales/i18nContext'
import { TRACKS_HOVER_LAYER } from '../../../map/layerIds'
import usePreview from '../../../map/usePreview'
import ReadOnlyField from '../../form/ReadOnlyField'
import FormSection from '../../form/FormSection'
import CommitBar from '../../form/CommitBar'
import { searchCrossover } from '../../../utils/crossover'
import SwitchFormField from './SwitchFormField'
import MidChainFields from './MidChainFields'
import PickPhase from './PickPhase'
import useConnectionPicks from './useConnectionPicks'
import { PREVIEW_LAYERS, PREVIEW_SOURCE, POINTS_SOURCE, ldsCants, pointsGeoJSON, previewGeoJSON } from './connectionPreview'

const fmt = (v, digits = 1) => v.toFixed(digits)

/** The speeds a connection can be held to: those the forms are built for. */
const SPEEDS = [...new Set(SWITCH_PICK_TYPES.map(f => f.speed).filter(Number.isFinite))].sort((a, b) => a - b)

/** The form the dialog starts on: the one the switch connection starts on (R 1200). */
const DEFAULT_FORM = Math.max(0, SWITCH_PICK_TYPES.indexOf(SWITCH_TYPES.find(f => f.R === 1200)))

const formIndex = (form) => Math.max(0, SWITCH_PICK_TYPES.findIndex(f => f.label === form?.label))

/** One middle element of an answer, as the dialog names it: type, length and an arc's radius. */
function pieceText(t, piece) {
  const name = t(`scurve_mid_type_${piece.type}`)
  return piece.type === 'arc'
    ? `${name} ${fmt(piece.length)} m · R ${fmt(1 / Math.abs(piece.kappa), 0)} m`
    : `${name} ${fmt(piece.length)} m`
}

/**
 * Does an answer bend each arc to the side the dialog fixed for it? A radius
 * typed with its sign fixes the side (+ right, − left, the way the
 * connection runs); the search tries a given radius either way.
 */
const keepsSides = (mids, sol) => mids.every((m, i) => m.type !== 'arc' || !m.side
  || Math.sign(-(sol.pieces[i]?.kappa ?? 0)) === m.side)

/**
 * The extended switch connection (decision 0018): two turnouts of any form
 * of the catalogue, each its own — laid into its track, so in a curve an
 * inner- or outer-bent turnout —, and one to three elements between their
 * branches. Its search (crossover/, in Rust as WebAssembly) moves both toes
 * along the elements picked and finds the elements between: every sequence
 * of one to three where they are left to it, else the ones set here, each
 * length and radius given or sought. It lists what closes, nearest to the
 * click first; the one taken is solved again here, shown on the map, and
 * built.
 *
 * `edit` ({ found, apart, start } of commands/crossoverEdit) opens it on a
 * connection that exists: its forms, its speed and its elements as they are,
 * the connection itself the one taken. `initialPicks` starts on two picks
 * made in the switch connection.
 */
export default function ExtendedConnectionForm({ onCommitted, edit = null, initialPicks = null }) {
  const { t, fill } = useI18n()
  const { phase, picks, pickStatus, reset: resetPicks } = useConnectionPicks({ initial: edit?.apart.picks ?? initialPicks })
  const [forms, setForms] = useState(edit ? edit.found.forms.map(formIndex) : [DEFAULT_FORM, DEFAULT_FORM])
  const [speedRaw, setSpeed] = useState(edit?.found.speed ?? null)
  // 'auto': every sequence of one to three elements; 'set': the ones set here.
  const [mode, setMode] = useState(edit && edit.found.mids.length > 1 ? 'set' : 'auto')
  const [mids, setMids] = useState(edit && edit.found.mids.length > 1
    ? edit.start.mids.map(m => ({ type: m.type, length: null, radius: null }))
    : [{ type: 'arc', length: null, radius: null }])
  const [carveError, setCarveError] = useState(null)

  const pair = forms.map(i => SWITCH_PICK_TYPES[i])
  const maxSpeed = Math.min(...pair.map(f => f.speed))
  const speeds = SPEEDS.filter(v => v <= maxSpeed)
  const speed = speedRaw != null && speedRaw <= maxSpeed ? speedRaw : maxSpeed

  // What a search is asked: a run and the answer taken belong to it, and
  // anything changed is a new question.
  const key = JSON.stringify([picks.map(p => [p?.trackId, p?.elIdx, p?.along]), forms, speed, mode, mode === 'set' ? mids : null])
  const [state, setState] = useState(null)        // { key, running | solutions | none | error }
  const [taken, setTaken] = useState(edit?.start.closes
    ? { key, index: -1, shift: edit.start.shift, mids: edit.start.mids, seed: edit.start.seed }
    : null)
  const run = state?.key === key ? state : null
  const chosen = taken?.key === key ? taken : null
  const ctl = useRef(null)
  useEffect(() => () => ctl.current?.abort(), [])

  const preview = usePreview(PREVIEW_LAYERS, { resetFilters: [TRACKS_HOVER_LAYER], resetCursor: true })
  const settled = useMemo(() => (
    chosen && picks[0] && picks[1]
      ? settleConnection({ picks, speed, shift: chosen.shift, mids: chosen.mids, seed: chosen.seed, form: pair })
      : null
    // `pair` follows `forms`, which the key holds
    // eslint-disable-next-line react-hooks/exhaustive-deps
  ), [chosen, picks, speed])
  const result = settled?.result ?? null
  const stands = !!(result?.valid || result?.overridable)
  useEffect(() => {
    if (stands) {
      preview.set(PREVIEW_SOURCE, previewGeoJSON(result))
      preview.set(POINTS_SOURCE, pointsGeoJSON(result))
    } else {
      preview.clear()
    }
  }, [result, stands, preview])

  // A toe's station from the begin of track 1, as the switch connection's slider states it.
  const stationOf = useMemo(() => {
    if (!picks[0] || !picks[1]) return () => 0
    const track1 = (edit?.apart.tracks ?? loadTracks()).find(tr => tr.id === picks[0].trackId)
    const elStart = track1 ? elementStations(track1)[picks[0].elIdx]?.start ?? 0 : 0
    const dir1 = stems(picks).g1.dir ?? 1
    return (shift) => elStart + picks[0].along + dir1 * shift
  }, [picks, edit])

  const search = async () => {
    ctl.current?.abort()
    const controller = new AbortController()
    ctl.current = controller
    setCarveError(null)
    setState({ key, running: true })
    const request = {
      ...crossoverRequest({ picks, speed, form: pair }),
      ...(mode === 'set' ? { elements: mids.map(m => ({ type: m.type, length: m.length, radius: m.radius })) } : {}),
    }
    try {
      const answer = await searchCrossover(request, { signal: controller.signal })
      const solutions = (answer?.solutions ?? []).filter(sol => mode !== 'set' || keepsSides(mids, sol))
      if (solutions.length) setState({ key, solutions })
      else if (!answer?.error || answer.error === 'no_solution') setState({ key, none: true })
      else setState({ key, error: answer.error })
    } catch (err) {
      if (err?.name !== 'AbortError') setState({ key, error: 'internal' })
    }
  }
  const cancelSearch = () => { ctl.current?.abort(); setState(null) }
  const take = (solution, index) => {
    const adopted = adoptCrossover({ picks, speed, solution, form: pair, lenient: true })
    if (!adopted) { setState(s => ({ ...s, failed: index })); return }
    setCarveError(null)
    setTaken({ key, index, ...adopted })
  }

  const reset = () => {
    preview.clear()
    resetPicks()
    setState(null)
    setTaken(null)
    setCarveError(null)
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

  if (phase !== 'config') {
    return <PickPhase phase={phase} picks={picks} pickStatus={pickStatus} onCancel={handleCancel} />
  }

  const setForm = (k, i) => setForms(prev => (k === 0 && prev[0] === prev[1] ? [i, i] : prev.map((v, j) => (j === k ? i : v))))
  const bauformText = (stemR, branchR) => t(`switch_bauform_${bauform(stemR, branchR)}`)

  return (
    <>
      <div className="element-form">
        <ReadOnlyField label={t('scurve_line1')} value={picks[0]?.name ?? ''} />
        <ReadOnlyField label={t('scurve_line2')} value={picks[1]?.name ?? ''} />
        <SwitchFormField label={t('xcon_form_1')} value={forms[0]} hideSymmetric onChange={i => setForm(0, i)} />
        <SwitchFormField label={t('xcon_form_2')} value={forms[1]} hideSymmetric onChange={i => setForm(1, i)} />
        <div className="form-field">
          <label>{t('field_speed')}</label>
          <select value={speed} onChange={e => setSpeed(Number(e.target.value))}>
            {speeds.map(v => <option key={v} value={v}>{`${v} km/h`}</option>)}
          </select>
        </div>

        <FormSection title={t('scurve_mids')}>
          <div className="form-field">
            <select value={mode} onChange={e => setMode(e.target.value)}>
              <option value="auto">{t('xcon_mode_auto')}</option>
              <option value="set">{t('xcon_mode_set')}</option>
            </select>
          </div>
        </FormSection>
        {mode === 'set' && (
          <MidChainFields mids={mids} onChange={setMids} title={t('xcon_mids_set')}
            status={{ text: t('xcon_mids_hint'), ok: true }}
            solved={result?.midPieces?.map(p => p.route)
              ?? (stands ? [{ length: result.Lg, r1: result.signedRg, r2: result.signedRg }] : null)} />
        )}
      </div>

      {!run?.running && (
        <button type="button" className="ui-btn ui-btn-block" onClick={search}>{t('xcon_search')}</button>
      )}
      {run?.running && (<>
        <p className="msg-info">{t('xcon_running')}</p>
        <button type="button" className="ui-btn ui-btn-block" onClick={cancelSearch}>{t('btn_cancel')}</button>
      </>)}
      {run?.none && <p className="msg-error">{t('xcon_none')}</p>}
      {run?.error && <p className="msg-error">{t(run.error === 'crossover_missing' ? 'xcon_missing' : 'xcon_failed')}</p>}
      {run?.solutions && (
        <ul className="crossover-solutions">
          {run.solutions.map((sol, i) => (
            <li key={`${sol.template.join('-')}-${i}`} className={`crossover-solution${chosen?.index === i ? ' is-chosen' : ''}`}>
              <div className="crossover-solution-text">
                <strong>{sol.pieces.map(p => pieceText(t, p)).join(' + ')}</strong>
                <span>{fill('xcon_station', {
                  st: fmt(stationOf(sol.s1), 2),
                  total: fmt(sol.pieces.reduce((sum, p) => sum + p.length, 0)),
                })}</span>
                {sol.hints.length > 0 && (
                  <span className="msg-hint">{fill('xcon_rw_hint', { rw: fmt(Math.min(...sol.hints), 0) })}</span>
                )}
                {run.failed === i && <span className="msg-error">{t('xcon_adopt_failed')}</span>}
              </div>
              <button type="button" className="ui-btn" onClick={() => take(sol, i)} disabled={chosen?.index === i}>
                {t(chosen?.index === i ? 'xcon_taken' : 'xcon_adopt')}
              </button>
            </li>
          ))}
        </ul>
      )}

      {chosen && result && (
        <div className="element-form">
          {chosen.index === -1 && <p className="msg-hint">{t('xcon_edit_as_is')}</p>}
          {stands && (<>
            <ReadOnlyField label={t('xcon_turnout_1')}
              value={`${result.switchType.label} · ${bauformText(result.stemR1, result.signedR1)}`} />
            <ReadOnlyField label={t('xcon_turnout_2')}
              value={`${(result.switchType2 ?? result.switchType).label} · ${bauformText(result.stemR2, result.signedR2)}`} />
            <ReadOnlyField label={t('scurve_station')} value={fmt(stationOf(settled.shift), 2)} />
            <ReadOnlyField label={t('scurve_total')} value={`${result.laenge.toFixed(2)} m`} />
          </>)}
          <p className={result.valid ? 'msg-info' : 'msg-error'}>
            {result.valid ? t('scurve_valid')
              : result.reason === 'cant_mismatch' ? fill('scurve_cant_mismatch', ldsCants(result))
                : t(REASON_MSG[result.reason] ?? 'scurve_invalid')}
          </p>
          {result.valid && result.midJumpHints?.map(h => (
            <p key={h.index} className="msg-hint">
              {fill('scurve_mid_jump_hint', {
                at: h.index < (result.midPieces?.length ?? 1)
                  ? fill('scurve_joint_start', { n: String(h.index + 1) })
                  : fill('scurve_joint_end', { n: String(result.midPieces?.length ?? 1) }),
                rw: h.rw.toFixed(0), reg: h.reg.toFixed(0),
              })}
            </p>
          ))}
          {result.short && (
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
            <p className="form-error">{fill('switch_on_track_no_room', { m: carveError.toFixed(1) })}</p>
          )}
        </div>
      )}

      <CommitBar onCommit={handleCommit} onCancel={handleCancel} disabled={!result?.valid}
        onOverride={result?.overridable ? () => handleCommit(true) : null} />
    </>
  )
}
