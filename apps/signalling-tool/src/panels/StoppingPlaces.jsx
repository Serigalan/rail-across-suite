import { useEffect, useState } from 'react'
import { Button, NumberField, Option, Options, Section, SelectField, TextField } from '@rail-across/ui'
import { newStoppingPlace, placeLength, placeWhere, snappedStation, stoppingPlacesOf, stoppingStretches, stretchAt } from '../model/stoppingPlaces.js'
import { trackName } from '../model/places.js'
import { stationName } from '../model/stations.js'
import { StopIcon } from '../icons.jsx'
import { EditorHead, ObjectFindings } from './common.jsx'

const byLabel = (a, b) => String(a.label).localeCompare(String(b.label), 'de', { numeric: true })

/** What a platform is called in a choice: where it lies. */
const platformText = (net, pf, t) => t('platform_at', { track: trackName(net, pf.trackId), from: Math.round(pf.startStation), to: Math.round(pf.endStation) })

/** A pick on the plan this component started ends where it is left. */
function useOwnMode(setMode, owner) {
  useEffect(() => () => setMode?.(m => (m?.owner === owner ? null : m)), [setMode, owner])
}

/**
 * The stopping places of a station — or, `entry` null, of the open line —:
 * the list, each opened in its editor, and a new one on a stretch of a track
 * or platform, as the signals and detection points cut it. A station offers
 * its tracks and its platforms (every track where none is its own), the open
 * line the platforms of no station. On the plan, as the Layout-Tool picks
 * a platform's track, a click on one of them takes the stretch around it.
 */
export function StoppingPlacesSection({ entry, domain, net, core, ops, select, mode, setMode, readOnly, t }) {
  const [source, setSource] = useState(null)
  const pointId = entry?.point.id ?? null
  const owner = `stopping-places:${pointId ?? 'open'}`
  useOwnMode(setMode, owner)
  const mine = stoppingPlacesOf(domain, pointId)
  const platforms = domain.collections.platforms.filter(pf => pf.trackId && Number.isFinite(pf.startStation) && Number.isFinite(pf.endStation))
  const coreOf = new Map((core.collections.platforms ?? []).map(p => [p.id, p.operationalPoint ?? null]))
  const ownPlatforms = entry ? platforms.filter(pf => entry.platforms.includes(pf.id)) : platforms.filter(pf => !coreOf.get(pf.id))
  const tracks = !entry ? [] : (entry.tracks.length ? net.tracks.filter(tr => entry.tracks.includes(tr.id)) : net.tracks)
  const sources = [
    ...ownPlatforms.map(pf => ({ value: `pf:${pf.id}`, label: platformText(net, pf, t) })).sort(byLabel),
    ...tracks.map(tr => ({ value: `tr:${tr.id}`, label: `${t('track')} ${trackName(net, tr.id)}` })).sort(byLabel),
  ]
  const [kind, id] = source ? source.split(/:(.*)/s) : []
  const platform = kind === 'pf' ? platforms.find(pf => pf.id === id) : null
  const trackId = platform?.trackId ?? (kind === 'tr' ? id : null)
  const stretches = trackId ? stoppingStretches(net, domain, trackId, t, platform ? [platform.startStation, platform.endStation] : []) : []
  const end = t(platform ? 'platform_end' : 'track_end')
  // Named by its track or platform; a second of the name counted on.
  const create = (s) => {
    const base = t(s.platformId ? 'stopping_name_platform' : 'stopping_name_track', { track: trackName(net, s.trackId) })
    const taken = new Set(mine.map(h => h.name))
    let name = base
    for (let n = 2; taken.has(name); n++) name = `${base} (${n})`
    const h = newStoppingPlace({ name, operationalPointId: pointId ?? undefined, platformId: s.platformId, trackId: s.trackId, from: s.from, to: s.to })
    ops.add('stoppingPlaces', h)
    setSource(null)
  }
  const picking = mode?.owner === owner
  const where = { tracks: tracks.map(tr => tr.id), platforms: ownPlatforms }
  const pickOnPlan = () => setMode(picking ? null : {
    kind: 'pick-stretch', owner, hint: t('stopping_pick_stretch'),
    stretchAt: (hit) => stretchAt(net, domain, where, hit, t),
    apply: (s) => { setMode(null); create(s) },
  })

  return (
    <Section title={t('stopping_places')}>
      <p className="ui-muted">{t(entry ? 'stopping_places_hint' : 'stopping_places_open_hint')}</p>
      {mine.length ? (
        <Options>
          {mine.map(h => (
            <Option key={h.id} icon={<StopIcon />} end={`${placeLength(h)} m`} onClick={() => select({ collection: 'stoppingPlaces', id: h.id })}>
              {h.name ?? t('unnamed')}<span className="ui-muted"> · {placeWhere(net, h)}</span>
            </Option>
          ))}
        </Options>
      ) : <p className="ui-list-empty">{t('stopping_places_none')}</p>}
      {!readOnly && <>
        <div className="ui-fields">
          <SelectField compact wide label={t('stopping_place_new')} value={source} empty={sources.length ? t('choose') : t('stopping_place_no_source')}
            options={sources} disabled={!sources.length} onChange={setSource} />
        </div>
        {setMode && sources.length > 0 && (
          <div className="ui-row">
            <Button size="small" active={picking} onClick={pickOnPlan}>{t('stopping_pick_on_plan')}</Button>
          </div>
        )}
        {trackId && (stretches.length ? <>
          <p className="ui-muted">{t('stopping_stretches_hint')}</p>
          <Options>
            {stretches.map(s => (
              <Option key={`${s.from}-${s.to}`} icon={<StopIcon />} end={`${Math.round(s.to - s.from)} m`} onClick={() => create({ ...s, trackId, platformId: platform?.id })}>
                {`${Math.round(s.from)}–${Math.round(s.to)} m`}<span className="ui-muted"> · {s.below ?? end} – {s.above ?? end}</span>
              </Option>
            ))}
          </Options>
        </> : <p className="ui-list-empty">{t('stopping_stretches_none')}</p>)}
      </>}
    </Section>
  )
}

/**
 * A stopping place: its name, its stretch, and where it belongs — a station,
 * a platform. Its two ends can be picked on the plan anew, on its track, as
 * the Layout-Tool picks a platform's: each at a signal or detection point
 * within reach.
 */
export function StoppingPlaceEditor({ place, domain, net, core, findings, ops, mode, setMode, readOnly, t }) {
  const set = (patch) => ops.update('stoppingPlaces', { ...place, ...patch })
  const setAt = (patch) => set({ at: { ...place.at, ...patch } })
  const owner = `stopping-ends:${place.id}`
  useOwnMode(setMode, owner)
  const picking = mode?.owner === owner
  const pickEnds = (first = null) => setMode({
    kind: 'pick-stretch', owner, hint: t(first == null ? 'stopping_pick_start' : 'stopping_pick_end'),
    stretchAt: (hit) => {
      if (hit?.trackId !== place.at.trackId) return null
      const s = snappedStation(domain, hit.trackId, hit.station, hit.reach, t)
      return { trackId: hit.trackId, from: first ?? s, to: s }
    },
    apply: ({ from, to }) => {
      if (first == null) { pickEnds(to); return }
      setMode(null)
      if (Math.abs(to - from) < 1) return
      ops.modify('stoppingPlaces', place.id, (now) => ({ ...now, at: { ...now.at, startStation: Math.min(from, to), endStation: Math.max(from, to) } }))
    },
  })
  const point = (core.collections.operationalPoints ?? []).find(p => p.id === place.operationalPointId)
  const platform = domain.collections.platforms.find(pf => pf.id === place.platformId)
  const itineraries = domain.collections.itineraries.filter(it => it.startStoppingPlaceId === place.id || it.targetStoppingPlaceId === place.id)
  return (
    <div className="tool-editor">
      <EditorHead title={place.name ?? t('unnamed')} readOnly={readOnly} t={t} onDelete={() => ops.remove('stoppingPlaces', place.id)} />
      <ObjectFindings findings={findings} collection="stoppingPlaces" id={place.id} t={t} />
      <div className="ui-fields">
        <TextField compact wide label={t('name')} value={place.name ?? ''} disabled={readOnly} onChange={(v) => set({ name: v || undefined })} />
        <NumberField compact label={t('stopping_from')} value={place.at.startStation} disabled={readOnly} onChange={(v) => v != null && setAt({ startStation: v })} />
        <NumberField compact label={t('stopping_to')} value={place.at.endStation} disabled={readOnly} onChange={(v) => v != null && setAt({ endStation: v })} />
      </div>
      {!readOnly && setMode && (
        <div className="ui-row">
          <Button size="small" active={picking} onClick={() => (picking ? setMode(null) : pickEnds())}>{t('stopping_pick_ends')}</Button>
        </div>
      )}
      <dl className="tool-facts">
        <dt>{t('track')}</dt><dd>{trackName(net, place.at.trackId)}</dd>
        <dt>{t('stopping_length')}</dt><dd>{placeLength(place)} m</dd>
        <dt>{t('object_operationalPoints')}</dt><dd>{point ? stationName(point) : t('open_line')}</dd>
        {platform && <><dt>{t('object_platforms')}</dt><dd>{platformText(net, platform, t)}</dd></>}
        <dt>{t('itineraries')}</dt><dd>{itineraries.length}</dd>
      </dl>
      <p className="ui-muted">{t('stopping_place_bounds_hint')}</p>
    </div>
  )
}

