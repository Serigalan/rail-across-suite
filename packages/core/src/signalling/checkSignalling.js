import { aspectOf, boardAspect, knownSystem, signalKindOf, symbolFits } from './catalogue.js'

/**
 * Whether the signalling domain of a project holds together, on its own and
 * on the core it stands on — what the schema cannot say (decision 0011):
 *
 * - every reference finds its target, in the domain or in the core;
 * - every location lies on its track, within the track's length;
 * - every signal shows what the catalogue lets its kind show: known aspects,
 *   on the frames they belong on, with the symbol they ask for; a screen with
 *   the aspects its kind must have and none it may not; indicators its kind
 *   carries; a virtual signal shows nothing;
 * - a route runs from the track of its start signal to that of its target, a
 *   train route ends in an overlap behind its target, at most one of them
 *   preferred, and shows only aspects its signals have;
 * - an overlap starts at its signal's track and ends on the last of its path;
 * - flank protection is asked for once per switch element and position, and
 *   never given by the element that asks for it;
 * - a PZB speed check has its test values and a magnet none, an element
 *   switched by its signal names it;
 * - a station boundary is marked by an entry signal, a board Ne 1 or a
 *   switch — anything else is a special case worth a warning (EBO §4).
 *
 * `core` and `signalling` are the two domains of a core 1.0 document
 * ({ fields, collections }). Pure, like validateProject. Returns
 * { errors, warnings }, each finding { code, key, collection, id, params },
 * `key` naming it stably (code and ids) for newFindings.
 *
 * Not yet checked, for want of the values (catalogue open_points): distances
 * — distant signal to main signal, the length an overlap needs, where the PZB
 * magnets lie. Not yet derived: the extent of a track section from its
 * detection points over the net.
 */
export function checkSignalling({ core, signalling }) {
  const errors = []
  const warnings = []
  const c = signalling?.collections ?? {}
  const say = (list, code, collection, id, params = {}, extra = '') => list.push({
    code, key: `${code}:${collection}:${id}${extra ? `:${extra}` : ''}`, collection, id, params,
  })
  const error = (...a) => say(errors, ...a)
  const warning = (...a) => say(warnings, ...a)

  const tracks = new Map((core?.collections?.tracks ?? []).map(t => [t.id, t]))
  const switches = new Set((core?.collections?.switches ?? []).map(s => s.id))
  const points = new Set((core?.collections?.operationalPoints ?? []).map(p => p.id))
  const stages = new Set((core?.fields?.stages ?? []).map(s => s.id))
  const ids = (name) => new Set((c[name] ?? []).map(o => o.id))
  const signals = new Map((c.signals ?? []).map(s => [s.id, s]))
  const known = {
    detectionPoints: ids('detectionPoints'), trackSections: ids('trackSections'), switchElements: ids('switchElements'),
    routes: ids('routes'), overlaps: ids('overlaps'), flankProtection: ids('flankProtection'),
  }
  const overlaps = new Map((c.overlaps ?? []).map(o => [o.id, o]))

  /** A reference that finds nothing. */
  const ref = (collection, id, target, value, field) => {
    if (value == null) return
    const there = target === 'core.tracks' ? tracks.has(value)
      : target === 'core.switches' ? switches.has(value)
        : target === 'core.operationalPoints' ? points.has(value)
          : target === 'signalling.signals' ? signals.has(value)
            : known[target.slice('signalling.'.length)].has(value)
    if (!there) error('reference_missing', collection, id, { field, target, ref: value }, `${field}:${value}`)
  }
  const lifespan = (collection, obj) => {
    for (const end of ['from', 'to']) {
      const stage = obj.lifespan?.[end]
      if (stage != null && !stages.has(stage)) error('reference_missing', collection, obj.id, { field: `lifespan.${end}`, target: 'core.fields.stages', ref: stage }, `lifespan.${end}`)
    }
  }
  const trackLength = (track) => (track.elements ?? []).reduce((sum, el) => sum + (el.length ?? 0), 0)
  /** A location: on a track that exists, within its length — or an orphan in the plane. */
  const at = (collection, obj, loc, field = 'at') => {
    if (!loc) return
    if (loc.anchor === 'plane') {
      warning('orphaned', collection, obj.id, { field }, field)
      return
    }
    ref(collection, obj.id, 'core.tracks', loc.trackId, `${field}.trackId`)
    const track = tracks.get(loc.trackId)
    if (track && loc.station != null && (loc.station < -STATION_TOL || loc.station > trackLength(track) + STATION_TOL)) {
      error('off_track', collection, obj.id, { field, station: loc.station, length: trackLength(track) }, field)
    }
  }
  const path = (collection, obj, p, field = 'path') => {
    for (const t of p?.trackIds ?? []) ref(collection, obj.id, 'core.tracks', t, `${field}.trackIds`)
    if (!p?.trackIds?.length) error('path_empty', collection, obj.id, { field }, field)
  }
  const trackOf = (signalId) => signals.get(signalId)?.at?.trackId

  // ── signals ──
  for (const s of c.signals ?? []) {
    at('signals', s, s.at)
    lifespan('signals', s)
    const frames = s.frames ?? []
    if (s.kind === 'virtual') {
      if (frames.length) error('virtual_with_frames', 'signals', s.id)
      if (!s.virtualFunction) warning('virtual_without_function', 'signals', s.id)
      continue
    }
    if (s.system && !knownSystem(s.system)) error('system_unknown', 'signals', s.id, { system: s.system })
    const screens = frames.filter(f => f.kind === 'screen')
    if (screens.length > 1) error('screens_many', 'signals', s.id, { n: screens.length })
    const kind = screens.length ? signalKindOf(s.system, s.type) : null
    if (screens.length && s.type && !kind && (!s.system || knownSystem(s.system))) {
      error('type_not_in_system', 'signals', s.id, { type: s.type, system: s.system ?? null })
    }
    if (screens.length && !s.type) error('type_missing', 'signals', s.id)
    for (const [i, frame] of frames.entries()) {
      for (const a of frame.aspects) {
        const aspect = aspectOf(a.aspect)
        const where = `${i}:${a.aspect}`
        if (!aspect) { error('aspect_unknown', 'signals', s.id, { aspect: a.aspect }, where); continue }
        if (!aspect.rahmen.includes(frame.kind)) { error('aspect_not_on_frame', 'signals', s.id, { aspect: a.aspect, frame: frame.kind }, where); continue }
        if (!symbolFits(aspect, a.symbol)) error('symbol_invalid', 'signals', s.id, { aspect: a.aspect, symbol: a.symbol ?? null }, where)
        if (frame.kind === 'indicator' && kind && !kind.zusatz.includes(a.aspect)) {
          error('indicator_not_allowed', 'signals', s.id, { aspect: a.aspect, type: s.type }, where)
        }
        if (frame.kind === 'plate' && !screens.length && !boardAspect(a.aspect)) {
          error('board_not_allowed', 'signals', s.id, { aspect: a.aspect }, where)
        }
      }
    }
    if (kind) {
      const shown = new Set(screens[0].aspects.map(a => a.aspect))
      for (const must of kind.schirm.pflicht) if (!shown.has(must)) error('screen_lacks', 'signals', s.id, { aspect: must, type: s.type }, must)
      for (const has of shown) if (!kind.schirm.erlaubt.includes(has)) error('screen_not_allowed', 'signals', s.id, { aspect: has, type: s.type }, has)
    }
  }

  // ── train detection, switch elements ──
  const bounding = new Set()
  for (const d of c.detectionPoints ?? []) { at('detectionPoints', d, d.at); lifespan('detectionPoints', d) }
  for (const t of c.trackSections ?? []) {
    for (const p of t.boundedBy) { ref('trackSections', t.id, 'signalling.detectionPoints', p, 'boundedBy'); bounding.add(p) }
    lifespan('trackSections', t)
  }
  for (const d of c.detectionPoints ?? []) if (!bounding.has(d.id)) warning('detection_point_unused', 'detectionPoints', d.id)
  for (const e of c.switchElements ?? []) {
    ref('switchElements', e.id, 'core.switches', e.switchId, 'switchId')
    lifespan('switchElements', e)
  }

  // ── routes and overlaps ──
  for (const r of c.routes ?? []) {
    ref('routes', r.id, 'signalling.signals', r.startSignalId, 'startSignalId')
    ref('routes', r.id, 'signalling.signals', r.targetSignalId, 'targetSignalId')
    path('routes', r, r.path)
    lifespan('routes', r)
    if (r.startSignalId === r.targetSignalId) error('route_to_itself', 'routes', r.id)
    const run = r.path?.trackIds ?? []
    if (run.length && trackOf(r.startSignalId) && run[0] !== trackOf(r.startSignalId)) error('route_start_off_path', 'routes', r.id)
    if (run.length && trackOf(r.targetSignalId) && run[run.length - 1] !== trackOf(r.targetSignalId)) error('route_target_off_path', 'routes', r.id)
    if (r.kind === 'train' && !r.trainType) warning('route_type_missing', 'routes', r.id)
    if (r.kind === 'shunting' && !r.shuntingType) warning('route_type_missing', 'routes', r.id)
    if (r.kind === 'shunting' && r.overlaps?.length) error('shunting_with_overlap', 'routes', r.id)
    if (r.kind === 'train' && signals.get(r.targetSignalId)?.kind === 'real' && !r.overlaps?.length) warning('route_without_overlap', 'routes', r.id)
    if ((r.overlaps ?? []).filter(o => o.preferred).length > 1) error('overlap_preferred_many', 'routes', r.id)
    for (const o of r.overlaps ?? []) {
      ref('routes', r.id, 'signalling.overlaps', o.overlapId, 'overlaps')
      const overlap = overlaps.get(o.overlapId)
      if (overlap && overlap.signalId !== r.targetSignalId) error('overlap_not_at_target', 'routes', r.id, { overlap: o.overlapId }, o.overlapId)
    }
    for (const a of r.aspects ?? []) {
      ref('routes', r.id, 'signalling.signals', a.signalId, 'aspects')
      const s = signals.get(a.signalId)
      const has = (s?.frames ?? []).some(f => f.aspects.some(x => x.aspect === a.aspect && (a.symbol === undefined || x.symbol === a.symbol)))
      if (s && !has) error('route_aspect_not_on_signal', 'routes', r.id, { signal: a.signalId, aspect: a.aspect, symbol: a.symbol ?? null }, `${a.signalId}:${a.aspect}`)
    }
  }
  for (const o of c.overlaps ?? []) {
    ref('overlaps', o.id, 'signalling.signals', o.signalId, 'signalId')
    path('overlaps', o, o.path)
    at('overlaps', o, o.end, 'end')
    lifespan('overlaps', o)
    const run = o.path?.trackIds ?? []
    if (run.length && trackOf(o.signalId) && run[0] !== trackOf(o.signalId)) error('overlap_start_off_path', 'overlaps', o.id)
    if (run.length && o.end?.anchor === 'station' && o.end.trackId !== run[run.length - 1]) error('overlap_end_off_path', 'overlaps', o.id)
    for (const t of o.clearSections ?? []) ref('overlaps', o.id, 'signalling.trackSections', t, 'clearSections')
    for (const sw of o.switches ?? []) ref('overlaps', o.id, 'signalling.switchElements', sw.switchElementId, 'switches')
  }

  // ── flank protection ──
  const asked = new Map()
  for (const f of c.flankProtection ?? []) {
    const { switchElementId, via } = f.requester
    ref('flankProtection', f.id, 'signalling.switchElements', switchElementId, 'requester')
    const key = `${switchElementId}:${via}`
    if (asked.has(key)) error('flank_asked_twice', 'flankProtection', f.id, { other: asked.get(key) }, key)
    else asked.set(key, f.id)
    if (f.protection.signalId) ref('flankProtection', f.id, 'signalling.signals', f.protection.signalId, 'protection')
    if (f.protection.switchElementId) {
      ref('flankProtection', f.id, 'signalling.switchElements', f.protection.switchElementId, 'protection')
      if (f.protection.switchElementId === switchElementId) error('flank_by_itself', 'flankProtection', f.id)
    }
    for (const side of ['left', 'right']) ref('flankProtection', f.id, 'signalling.flankProtection', f.passOn?.[side], `passOn.${side}`)
    for (const t of f.clearSections ?? []) ref('flankProtection', f.id, 'signalling.trackSections', t, 'clearSections')
    lifespan('flankProtection', f)
  }

  // ── PZB ──
  for (const p of c.pzbElements ?? []) {
    at('pzbElements', p, p.at)
    ref('pzbElements', p.id, 'signalling.signals', p.signalId, 'signalId')
    for (const r of p.routes ?? []) ref('pzbElements', p.id, 'signalling.routes', r.routeId, 'routes')
    if (p.kind === 'speedCheck' && !p.speedCheck) error('speed_check_values_missing', 'pzbElements', p.id)
    if (p.kind === 'magnet' && p.speedCheck) error('magnet_with_speed_check', 'pzbElements', p.id)
    if (p.effect === 'switchedBySignal' && !p.signalId) error('pzb_switched_without_signal', 'pzbElements', p.id)
    lifespan('pzbElements', p)
  }

  // ── station boundaries ──
  for (const b of c.stationBoundaries ?? []) {
    ref('stationBoundaries', b.id, 'core.operationalPoints', b.operationalPointId, 'operationalPointId')
    for (const m of b.marks) {
      if (m.switchId) { ref('stationBoundaries', b.id, 'core.switches', m.switchId, 'marks'); continue }
      ref('stationBoundaries', b.id, 'signalling.signals', m.signalId, 'marks')
      const s = signals.get(m.signalId)
      const entry = ['entry', 'entryExit', 'entryBlock'].includes(s?.function)
      const board = (s?.frames ?? []).some(f => f.aspects.some(a => a.aspect === 'Ne_1'))
      if (s && !entry && !board) warning('boundary_mark_special', 'stationBoundaries', b.id, { signal: m.signalId }, m.signalId)
    }
    lifespan('stationBoundaries', b)
  }

  return { errors, warnings }
}

/** Stations may overshoot the track length by this much [m], as validateProject allows. */
const STATION_TOL = 0.01
