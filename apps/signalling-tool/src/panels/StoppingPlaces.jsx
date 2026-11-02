import { useState } from 'react'
import { NumberField, Option, Options, Section, SelectField, TextField } from '@rail-across/ui'
import { newStoppingPlace, placeLength, placeWhere, stoppingPlacesOf, stoppingStretches } from '../model/stoppingPlaces.js'
import { trackName } from '../model/places.js'
import { stationName } from '../model/stations.js'
import { StopIcon } from '../icons.jsx'
import { EditorHead, ObjectFindings } from './common.jsx'

const byLabel = (a, b) => String(a.label).localeCompare(String(b.label), 'de', { numeric: true })

/** What a platform is called in a choice: where it lies. */
const platformText = (net, pf, t) => t('platform_at', { track: trackName(net, pf.trackId), from: Math.round(pf.startStation), to: Math.round(pf.endStation) })

/**
 * The stopping places of a station — or, `entry` null, of the open line —:
 * the list, each opened in its editor, and a new one on a stretch of a track
 * or platform, as the signals and detection points cut it. A station offers
 * its tracks and its platforms (every track where none is its own), the open
 * line the platforms of no station.
 */
export function StoppingPlacesSection({ entry, domain, net, core, ops, select, readOnly, t }) {
  const [source, setSource] = useState(null)
  const pointId = entry?.point.id ?? null
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
    const base = t(platform ? 'stopping_name_platform' : 'stopping_name_track', { track: trackName(net, trackId) })
    const taken = new Set(mine.map(h => h.name))
    let name = base
    for (let n = 2; taken.has(name); n++) name = `${base} (${n})`
    const h = newStoppingPlace({ name, operationalPointId: pointId ?? undefined, platformId: platform?.id, trackId, from: s.from, to: s.to })
    ops.add('stoppingPlaces', h)
    setSource(null)
  }

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
        {trackId && (stretches.length ? <>
          <p className="ui-muted">{t('stopping_stretches_hint')}</p>
          <Options>
            {stretches.map(s => (
              <Option key={`${s.from}-${s.to}`} icon={<StopIcon />} end={`${Math.round(s.to - s.from)} m`} onClick={() => create(s)}>
                {`${Math.round(s.from)}–${Math.round(s.to)} m`}<span className="ui-muted"> · {s.below ?? end} – {s.above ?? end}</span>
              </Option>
            ))}
          </Options>
        </> : <p className="ui-list-empty">{t('stopping_stretches_none')}</p>)}
      </>}
    </Section>
  )
}

/** A stopping place: its name, its stretch, and where it belongs — a station, a platform. */
export function StoppingPlaceEditor({ place, domain, net, core, findings, ops, readOnly, t }) {
  const set = (patch) => ops.update('stoppingPlaces', { ...place, ...patch })
  const setAt = (patch) => set({ at: { ...place.at, ...patch } })
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

