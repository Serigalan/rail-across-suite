import {
  SIGNAL_CATALOGUE, aspectFitsFrame, aspectOf, baliseKindOf, boardAspect, knownSystem, protectionSystemOf, settingsProblems, signalKindOf, symbolFits,
} from './catalogue.js'
import { overlapLength, signallingSpeedOf } from './overlaps.js'
import { trackSectionExtents } from './trackSections.js'
import { tracksOnFrom } from '../track/index.js'

/**
 * Whether the signalling domain of a project holds together, on its own and
 * on the core it stands on — what the schema cannot say (decision 0011):
 *
 * - every reference finds its target, in the domain or in the core;
 * - every location lies on its track, within the track's length;
 * - every logical signal of a signal is of a system and kind the catalogue
 *   knows, each system once, its settings and parameters as the system
 *   declares them, the systems it announces known;
 * - every logical signal shows what the catalogue lets its kind show: known
 *   aspects, on the frames they belong on, with the symbol they ask for; one
 *   screen with the aspects its kind must have and none it may not, a proceed
 *   aspect where its kind has some; indicators its kind carries; a place shows
 *   its boards, and a screen of its own only without logical signals; a
 *   virtual signal shows nothing;
 * - the speed a logical signal signals is for a route from its place, once;
 * - an overlap is as long as its speed — the one it allows, else the one its
 *   route leads up with — and its gradient ask (catalogue: durchrutschweg, a
 *   draft);
 * - a track detection section's points bound one part of the net, exactly;
 * - a route runs from the track of its start signal to that of its target, a
 *   train route ends in an overlap behind its target, at most one of them
 *   preferred, and shows only aspects its signals have;
 * - an overlap starts at its signal's track and ends on the last of its path;
 * - flank protection is asked for once per switch element and position, and
 *   never given by the element that asks for it;
 * - a balise or magnet is of a train protection system and kind the
 *   catalogue knows, has the settings its kind asks for and no others, each
 *   fitting; one switched by its signal names it;
 * - a station boundary is marked by an entry signal, a board Ne 1, a switch
 *   or a platform — anything else is a special case worth a warning (EBO §4).
 *
 * `core` and `signalling` are the two domains of a core 1.0 document
 * ({ fields, collections }). Pure, like validateProject. Returns
 * { errors, warnings }, each finding { code, key, collection, id, params },
 * `key` naming it stably (code and ids) for newFindings.
 *
 * Not yet checked, for want of the values (catalogue open_points): distances
 * — distant signal to main signal, where the balises and magnets lie.
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
  const platforms = new Set((core?.collections?.platforms ?? []).map(p => p.id))
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
          : target === 'core.platforms' ? platforms.has(value)
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
  /** Whether track `b` meets track `a` at an end: at one switch, or laid on from it. */
  const coreSwitches = core?.collections?.switches ?? []
  const beside = (a, b) => coreSwitches.some(sw => {
    const held = Object.values(sw.ports ?? {}).map(p => p?.trackId)
    return held.includes(a) && held.includes(b)
  }) || ['BEGIN', 'END'].some(end => tracksOnFrom(a, end, core?.collections?.tracks ?? [], coreSwitches).some(t => t.trackId === b))

  // ── signals: their places, their logical signals ──
  const routesById = new Map((c.routes ?? []).map(r => [r.id, r]))
  const isScreen = (f) => f.kind === 'screen'
  /** The aspects of frames: known, on the frames they belong on, with their symbols; indicators of `kind`, boards that stand alone. */
  const frameChecks = (s, frames, field, kind, anyScreen) => {
    for (const [i, frame] of frames.entries()) {
      if (!frame.aspects.length && frame.kind !== 'nameplate') warning('frame_empty', 'signals', s.id, { frame: frame.kind }, `${field}.${i}`)
      for (const a of frame.aspects) {
        const aspect = aspectOf(a.aspect)
        const where = `${field}.${i}:${a.aspect}`
        if (!aspect) { error('aspect_unknown', 'signals', s.id, { aspect: a.aspect }, where); continue }
        if (!aspectFitsFrame(aspect, frame.kind)) { error('aspect_not_on_frame', 'signals', s.id, { aspect: a.aspect, frame: frame.kind }, where); continue }
        if (!symbolFits(aspect, a.symbol)) error('symbol_invalid', 'signals', s.id, { aspect: a.aspect, symbol: a.symbol ?? null }, where)
        if (frame.kind === 'indicator' && kind && !kind.zusatz.includes(a.aspect)) error('indicator_not_allowed', 'signals', s.id, { aspect: a.aspect }, where)
        if (frame.kind === 'plate' && !anyScreen && !boardAspect(a.aspect)) error('board_not_allowed', 'signals', s.id, { aspect: a.aspect }, where)
      }
    }
  }
  for (const s of c.signals ?? []) {
    at('signals', s, s.at)
    for (const [i, also] of (s.alsoAt ?? []).entries()) at('signals', s, also, `alsoAt.${i}`)
    lifespan('signals', s)
    const own = s.frames ?? []
    const logical = s.logicalSignals ?? []
    if (s.kind === 'virtual') {
      if (own.length || logical.length) error('virtual_with_frames', 'signals', s.id)
      if (!s.virtualFunction) warning('virtual_without_function', 'signals', s.id)
      continue
    }
    const anyScreen = own.some(isScreen) || logical.some(l => (l.frames ?? []).some(isScreen))
    // The place: its boards; a screen of its own only where it has no logical signal to show it.
    if (logical.length) {
      for (const [i, f] of own.entries()) if (f.kind === 'screen' || f.kind === 'indicator') error('frame_of_logical_signal', 'signals', s.id, { frame: f.kind }, `frames.${i}`)
    }
    if (own.filter(isScreen).length > 1) error('screens_many', 'signals', s.id, { n: own.filter(isScreen).length })
    frameChecks(s, own, 'frames', null, anyScreen)
    const seen = new Set()
    for (const [i, l] of logical.entries()) {
      const where = `logicalSignals.${i}`
      if (seen.has(l.system)) error('system_twice', 'signals', s.id, { system: l.system }, where)
      seen.add(l.system)
      if (!knownSystem(l.system)) { error('system_unknown', 'signals', s.id, { system: l.system }, where); continue }
      if (l.type && l.type !== 'other' && !signalKindOf(l.system, l.type)) error('type_not_in_system', 'signals', s.id, { type: l.type, system: l.system }, where)
      const system = SIGNAL_CATALOGUE.systeme[l.system]
      for (const p of settingsProblems(l.settings, system.einstellungen)) {
        error(`setting_${p.problem}`, 'signals', s.id, { setting: p.setting, system: l.system }, `${where}:${p.setting}`)
      }
      for (const next of l.nextSystems ?? []) {
        if (!knownSystem(next)) error('system_unknown', 'signals', s.id, { system: next }, `${where}:next:${next}`)
      }
      const parameterSets = [l.defaultParameters, ...(l.conditionalParameters ?? []).map(p => p.parameters)]
      for (const [k, parameters] of parameterSets.entries()) {
        for (const p of settingsProblems(parameters, system.parameter)) {
          error(`parameter_${p.problem}`, 'signals', s.id, { parameter: p.setting, system: l.system }, `${where}:${k}:${p.setting}`)
        }
      }
      for (const p of l.conditionalParameters ?? []) ref('signals', s.id, 'signalling.routes', p.routeId, `${where}.conditionalParameters`)
      // What it shows: one screen, holding what its kind must and may show; indicators its kind carries.
      const frames = l.frames ?? []
      const screens = frames.filter(isScreen)
      const kind = signalKindOf(l.system, l.type)
      if (screens.length > 1) error('screens_many', 'signals', s.id, { n: screens.length }, where)
      if (screens.length && !l.type) error('type_missing', 'signals', s.id, { system: l.system }, where)
      frameChecks(s, frames, `${where}.frames`, kind, anyScreen)
      if (kind && screens.length) {
        const shown = new Set(screens[0].aspects.map(a => a.aspect))
        for (const must of kind.schirm.pflicht) if (!shown.has(must)) error('screen_lacks', 'signals', s.id, { aspect: must, system: l.system }, `${where}:${must}`)
        const proceed = kind.schirm.fahrt ?? []
        if (proceed.length && !proceed.some(a => shown.has(a))) warning('screen_no_proceed', 'signals', s.id, { aspects: proceed, system: l.system }, where)
        for (const has of shown) if (!kind.schirm.erlaubt.includes(has)) error('screen_not_allowed', 'signals', s.id, { aspect: has, system: l.system }, `${where}:${has}`)
      }
      // The speeds it signals: each for a route from its place, once.
      const speeds = new Set()
      for (const v of l.signallingSpeeds ?? []) {
        ref('signals', s.id, 'signalling.routes', v.routeId, `${where}.signallingSpeeds`)
        const route = routesById.get(v.routeId)
        if (route && route.startSignalId !== s.id) error('speed_route_elsewhere', 'signals', s.id, { route: v.routeId }, `${where}:${v.routeId}`)
        if (speeds.has(v.routeId)) error('speed_route_twice', 'signals', s.id, { route: v.routeId }, `${where}:${v.routeId}:twice`)
        speeds.add(v.routeId)
      }
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
  if (c.trackSections?.length) {
    for (const [id, extent] of trackSectionExtents({ core, signalling })) {
      if (!extent.matches) warning('section_extent_differs', 'trackSections', id, { missing: extent.missing })
    }
  }
  const switchElements = new Map((c.switchElements ?? []).map(e => [e.id, e]))
  for (const e of c.switchElements ?? []) {
    ref('switchElements', e.id, 'core.switches', e.switchId, 'switchId')
    if (e.derailer) at('switchElements', e, e.derailer.at, 'derailer.at')
    ref('switchElements', e.id, 'signalling.signals', e.clearanceMarkerId, 'clearanceMarkerId')
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
    if (r.kind === 'middle' && !r.middleType) warning('route_type_missing', 'routes', r.id)
    for (const t of r.clearSections ?? []) ref('routes', r.id, 'signalling.trackSections', t, 'clearSections')
    if (r.kind === 'shunting' && r.overlaps?.length) error('shunting_with_overlap', 'routes', r.id)
    if (r.kind === 'train' && signals.get(r.targetSignalId)?.kind === 'real' && !r.overlaps?.length) warning('route_without_overlap', 'routes', r.id)
    if ((r.overlaps ?? []).filter(o => o.preferred).length > 1) error('overlap_preferred_many', 'routes', r.id)
    const speed = r.kind === 'train' ? signallingSpeedOf(signalling, r) : null
    for (const o of r.overlaps ?? []) {
      ref('routes', r.id, 'signalling.overlaps', o.overlapId, 'overlaps')
      const overlap = overlaps.get(o.overlapId)
      if (overlap && overlap.signalId !== r.targetSignalId) error('overlap_not_at_target', 'routes', r.id, { overlap: o.overlapId }, o.overlapId)
      // As long as its speed and the gradient over it ask (catalogue: durchrutschweg): the speed it allows
      // where it states one (PlanPro DWeg_V), else the one the route leads up with.
      const governing = overlap?.speed ?? speed
      const needed = overlap && overlapLength(governing, overlap.gradient ?? 0)
      if (needed != null && overlap.length < needed * (1 - OVERLAP_TOL)) {
        warning('overlap_short', 'overlaps', overlap.id, { length: overlap.length, needed, speed: governing, route: r.id }, r.id)
      }
    }
    for (const a of r.aspects ?? []) {
      ref('routes', r.id, 'signalling.signals', a.signalId, 'aspects')
      const s = signals.get(a.signalId)
      const shown = [...(s?.frames ?? []), ...(s?.logicalSignals ?? []).flatMap(l => l.frames ?? [])]
      const has = shown.some(f => f.aspects.some(x => x.aspect === a.aspect && (a.symbol === undefined || x.symbol === a.symbol)))
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
    // Its end is its danger point: on its last track, or the clearance marker
    // of a switch at its end, on a track that leaves it there.
    if (run.length && o.end?.anchor === 'station' && !run.includes(o.end.trackId) && !beside(run[run.length - 1], o.end.trackId)) {
      error('overlap_end_off_path', 'overlaps', o.id)
    }
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
      const protecting = switchElements.get(f.protection.switchElementId)
      if (protecting?.switchId && !f.protection.position) error('flank_position_missing', 'flankProtection', f.id)
    }
    for (const side of ['left', 'right']) ref('flankProtection', f.id, 'signalling.flankProtection', f.passOn?.[side], `passOn.${side}`)
    for (const t of f.clearSections ?? []) ref('flankProtection', f.id, 'signalling.trackSections', t, 'clearSections')
    lifespan('flankProtection', f)
  }

  // ── balises and magnets ──
  for (const b of c.balises ?? []) {
    at('balises', b, b.at)
    ref('balises', b.id, 'signalling.signals', b.signalId, 'signalId')
    for (const r of b.routes ?? []) ref('balises', b.id, 'signalling.routes', r.routeId, 'routes')
    if (b.effect === 'switchedBySignal' && !b.signalId) error('balise_switched_without_signal', 'balises', b.id)
    lifespan('balises', b)
    const system = protectionSystemOf(b.system)
    if (!system) { error('system_unknown', 'balises', b.id, { system: b.system }); continue }
    const kind = baliseKindOf(b.system, b.kind)
    if (!kind) { error('kind_not_in_system', 'balises', b.id, { kind: b.kind, system: b.system }); continue }
    for (const p of settingsProblems(b.settings, system.einstellungen, kind)) {
      error(`setting_${p.problem}`, 'balises', b.id, { setting: p.setting, system: b.system }, p.setting)
    }
  }

  // ── station boundaries ──
  for (const b of c.stationBoundaries ?? []) {
    ref('stationBoundaries', b.id, 'core.operationalPoints', b.operationalPointId, 'operationalPointId')
    for (const m of b.marks) {
      if (m.switchId) { ref('stationBoundaries', b.id, 'core.switches', m.switchId, 'marks'); continue }
      if (m.platformId) { ref('stationBoundaries', b.id, 'core.platforms', m.platformId, 'marks'); continue }
      ref('stationBoundaries', b.id, 'signalling.signals', m.signalId, 'marks')
      const s = signals.get(m.signalId)
      const entry = [s?.function, ...(s?.logicalSignals ?? []).map(l => l.function)].some(fn => ['entry', 'entryExit', 'entryBlock'].includes(fn))
      const board = (s?.frames ?? []).some(f => f.aspects.some(a => a.aspect === 'Ne_1'))
      if (s && !entry && !board) warning('boundary_mark_special', 'stationBoundaries', b.id, { signal: m.signalId }, m.signalId)
    }
    lifespan('stationBoundaries', b)
  }

  return { errors, warnings }
}

/** An overlap may fall short of the length it needs by this share: a length measured off a plan rounds (199 m for 200). */
const OVERLAP_TOL = 0.01

/** Stations may overshoot the track length by this much [m], as validateProject allows. */
const STATION_TOL = 0.01
