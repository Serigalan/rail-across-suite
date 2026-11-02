import { parseXml, child, childrenNamed, value } from './xml.js'
import {
  BUFFER_STOPS, COORDINATE_SYSTEM, DERAILER_POSITION, DETECTION_POINT, FRAME, GEO_FORM, GUE_ARRANGEMENT, MOUNTING,
  OPERATION, POSITION, PZB_EFFECT, PZB_FREQUENCY, PZB_ROUTE_EFFECT, SECTION_KIND, SIDE, SIGNAL_FUNCTION, SIGNAL_SYSTEM,
  SIGNAL_TYPE, SWITCH_KIND, TWOFOLD_KIND, TWOFOLD_MEASURE, VIA, VIRTUAL_FUNCTION,
} from './mapping.js'
import { arcElement, reverseElement, straightElement, transitionElement } from '../geometry/index.js'
import { carryDomain } from '../merge/index.js'
import { defaultStages } from '../document/index.js'
import { CATALOGUE_REF } from '../signalling/index.js'

/**
 * Reading a PlanPro 1.10 plan (decision 0011): the net it is drawn on and its
 * signalling, as the suite's record holds them.
 *
 * PlanPro gives the net as a topology — edges (TOP_Kante) between nodes
 * (TOP_Knoten) — and its geometry — GEO_Kanten of a form, a length, radii
 * and a bearing between GEO_Knoten with coordinates. Every edge becomes a
 * track of the core, its elements the GEO_Kanten along it in the edge's
 * direction (A to B); every node where three edges meet (tip, left, right) a
 * turnout. So a point object at `Abstand` along an edge stands at that
 * station on its track, `Wirkrichtung` in is up, gegen down.
 *
 * A plan has a start and a target state (LST_Zustand_Start, _Ziel). What is in
 * both is existing, what only the target has is new, what only the start has
 * is removed — the two construction stages of the record.
 *
 * Whatever has no place in the record — level crossings, blocks, local control
 * zones, keys, the plan's attachments — is counted in the report, not read.
 */

const lower = (s) => (typeof s === 'string' ? s.toLowerCase() : s)
const num = (s) => {
  if (s == null || s === '') return undefined
  const n = Number(String(s).replace(',', '.'))
  return Number.isFinite(n) ? n : undefined
}
const bool = (s) => (s === 'true' ? true : s === 'false' ? false : undefined)
const withoutUndefined = (obj) => Object.fromEntries(Object.entries(obj).filter(([, v]) => v !== undefined))
const nonEmpty = (obj) => (obj && Object.keys(obj).length ? obj : undefined)
const GON = 0.9
const TRAIN_TYPES = new Set(['B', 'ZH', 'ZR', 'ZT', 'ZTU', 'ZU', 'ZUH', 'ZZ'])
const SHUNTING_TYPES = new Set(['RR', 'RT', 'RTU', 'RU'])
const MIDDLE_TYPES = new Set(['ZM', 'ZUM'])
/** A value of a PlanPro enumeration the schema has, else none. */
const known = (set, v) => (set.has(v) ? v : undefined)

/**
 * The objects of a plan: { header, objects: Map id → { tag, el, start, target } }
 * — `el` the target state's where it has one. Ids in lower case, as the core
 * keeps them.
 */
export function readPlanPro(text) {
  const root = parseXml(text)
  if (!/PlanPro_Schnittstelle$/.test(root.name)) throw new Error('planpro: not a PlanPro interface file')
  const objects = new Map()
  const take = (state, container) => {
    for (const el of container?.children ?? []) {
      const id = lower(value(el, 'Identitaet'))
      if (!id) continue
      const o = objects.get(id) ?? { tag: el.name, el, start: false, target: false }
      o[state] = true
      if (state === 'target') o.el = el
      objects.set(id, o)
    }
  }
  const fachdaten = childrenNamed(child(child(root, 'LST_Planung'), 'Fachdaten'), 'Ausgabe_Fachdaten')
  for (const f of fachdaten) {
    for (const c of childrenNamed(child(f, 'LST_Zustand_Start'), 'Container')) take('start', c)
    for (const c of childrenNamed(child(f, 'LST_Zustand_Ziel'), 'Container')) take('target', c)
  }
  const project = child(child(child(root, 'LST_Planung'), 'Objektmanagement'), 'LST_Planung_Projekt')
  const group = child(project, 'LST_Planung_Gruppe')
  const header = withoutUndefined({
    title: value(child(project, 'Planung_P_Allg'), 'Bezeichnung_Planung_Projekt'),
    projectNumber: value(child(project, 'Planung_P_Allg'), 'Projekt_Nummer'),
    installation: value(child(group, 'Planung_G_Schriftfeld'), 'Bezeichnung_Anlage'),
    subInstallation: value(child(group, 'Planung_G_Schriftfeld'), 'Bezeichnung_Unteranlage'),
    xsdVersion: value(child(group, 'Planung_G_Allg'), 'PlanPro_XSD_Version'),
    tool: [value(child(root, 'PlanPro_Schnittstelle_Allg'), 'Werkzeug_Name'), value(child(root, 'PlanPro_Schnittstelle_Allg'), 'Werkzeug_Version')].filter(Boolean).join(' ') || undefined,
    createdAt: value(child(root, 'PlanPro_Schnittstelle_Allg'), 'Erzeugung_Zeitstempel'),
  })
  return { header, objects }
}

/** What the import of a plan read, left out, and could not place. */
function newReport() {
  const counts = new Map()
  return {
    notes: [],
    skipped: counts,
    note(code, params = {}) { this.notes.push({ code, ...params }) },
    skip(tag) { counts.set(tag, (counts.get(tag) ?? 0) + 1) },
    summary() {
      return { notes: this.notes, skipped: Object.fromEntries([...counts].sort((a, b) => b[1] - a[1])) }
    },
  }
}

/** The net and the signalling of a plan's objects, for `projectId`. */
function convert(model, projectId) {
  const { objects } = model
  const report = newReport()
  const of = (tag) => [...objects.values()].filter(o => o.tag === tag)
  const get = (id) => objects.get(lower(id))
  const idOf = (el) => lower(value(el, 'Identitaet'))
  const stages = defaultStages(projectId)
  const final = stages[1].id
  /** The lifespan of an object by the states it is in; none for one in both. */
  const lifespanOf = (o) => (o.start && o.target ? undefined : o.target ? { from: final } : { to: final })
  const statusOf = (o) => (o.start && o.target ? undefined : o.target ? 'new' : 'removal')

  // ── geometry: GEO_Knoten → plane point ──
  const pointOfNode = new Map()
  let epsg = null
  for (const o of of('GEO_Punkt')) {
    const node = lower(value(o.el, 'ID_GEO_Knoten'))
    if (!node || pointOfNode.has(node)) continue
    const system = value(o.el, 'GEO_Punkt_Allg/GEO_Koordinatensystem')
    const code = COORDINATE_SYSTEM[system]
    if (!code) { report.note('coordinate_system_unknown', { system }); continue }
    if (epsg == null) epsg = code
    if (code !== epsg) { report.note('coordinate_systems_many', { system }); continue }
    const easting = num(value(o.el, 'GEO_Punkt_Allg/GK_X'))
    const northing = num(value(o.el, 'GEO_Punkt_Allg/GK_Y'))
    if (easting != null && northing != null) pointOfNode.set(node, { easting, northing, zone: code })
  }
  const geoByOwner = new Map()
  for (const o of of('GEO_Kante')) {
    const owner = lower(value(o.el, 'ID_GEO_Art'))
    if (!geoByOwner.has(owner)) geoByOwner.set(owner, [])
    geoByOwner.get(owner).push(o.el)
  }

  /** One GEO_Kante as an element of the core, in its own direction (node A to B). */
  const geoElement = (g) => {
    const a = pointOfNode.get(lower(value(g, 'ID_GEO_Knoten_A')))
    const b = pointOfNode.get(lower(value(g, 'ID_GEO_Knoten_B')))
    if (!a || !b) return null
    const formName = value(g, 'GEO_Kante_Allg/GEO_Form')
    const form = GEO_FORM[formName] ?? { type: 0 }
    if (!GEO_FORM[formName]) report.note('geo_form_as_straight', { form: formName })
    const length = num(value(g, 'GEO_Kante_Allg/GEO_Laenge'))
    const bearing = (num(value(g, 'GEO_Kante_Allg/GEO_Richtungswinkel')) ?? 0) * GON
    const rA = num(value(g, 'GEO_Kante_Allg/GEO_Radius_A')) || null
    const rB = num(value(g, 'GEO_Kante_Allg/GEO_Radius_B')) || null
    const gap = (x, y) => Math.abs(((x - y + 540) % 360) - 180)
    if (form.type === 0 || !(length > 0)) return straightElement(a, b)
    if (form.type === 1 && rA) {
      // Which way it bends is read off the bearing it starts at.
      const right = arcElement(a, b, Math.abs(rA)), left = arcElement(a, b, -Math.abs(rA))
      return gap(right.bearing, bearing) <= gap(left.bearing, bearing) ? right : left
    }
    // A transition: the signs that bring it, from its bearing, nearest its end point.
    let best = null
    for (const [sa, sb] of [[1, 1], [-1, -1], [1, -1], [-1, 1]]) {
      const r1 = rA ? sa * Math.abs(rA) : null, r2 = rB ? sb * Math.abs(rB) : null
      const { endUtm } = transitionElement(a, bearing, length, r1, r2, { transitionType: form.transitionType ?? 'clothoid' })
      const miss = Math.hypot(endUtm.easting - b.easting, endUtm.northing - b.northing)
      if (!best || miss < best.miss) best = { r1, r2, miss }
      if (!rA || !rB) { if (sa === -1) break }
    }
    if (best.miss > 0.05) report.note('geo_transition_misses_end', { id: idOf(g), miss: Math.round(best.miss * 1000) / 1000 })
    return transitionElement(a, bearing, length, best.r1, best.r2, { transitionType: form.transitionType ?? 'clothoid', endUtm: b }).element
  }

  // ── the net: an edge is a track ──
  const edges = new Map()   // edge id → { a, b, connA, connB, length }
  for (const o of of('TOP_Kante')) {
    edges.set(idOf(o.el), {
      o, a: lower(value(o.el, 'ID_TOP_Knoten_A')), b: lower(value(o.el, 'ID_TOP_Knoten_B')),
      connA: value(o.el, 'TOP_Kante_Allg/TOP_Anschluss_A'), connB: value(o.el, 'TOP_Kante_Allg/TOP_Anschluss_B'),
      length: num(value(o.el, 'TOP_Kante_Allg/TOP_Laenge')),
    })
  }
  const geoNodeOf = (topNode) => lower(value(get(topNode)?.el, 'ID_GEO_Knoten'))
  const names = new Map()   // edge id → track name (Gleis_Bezeichnung), its line (Strecke)
  const lines = new Map()
  const covering = (tag, into, read) => {
    for (const o of of(tag)) {
      const v = read(o.el)
      if (v == null) continue
      for (const part of childrenNamed(o.el, 'Bereich_Objekt_Teilbereich')) {
        const edge = lower(value(part, 'ID_TOP_Kante'))
        if (edge && !into.has(edge)) into.set(edge, v)
      }
    }
  }
  covering('Gleis_Bezeichnung', names, el => value(el, 'Bezeichnung/Bez_Gleis_Bezeichnung'))
  covering('Strecke', lines, el => value(el, 'Bezeichnung/Bezeichnung_Strecke'))

  const tracks = []
  for (const [id, e] of edges) {
    const geos = geoByOwner.get(id) ?? []
    let at = geoNodeOf(e.a)
    const end = geoNodeOf(e.b)
    const left = new Set(geos)
    const elements = []
    while (at && at !== end && left.size) {
      const next = [...left].find(g => lower(value(g, 'ID_GEO_Knoten_A')) === at || lower(value(g, 'ID_GEO_Knoten_B')) === at)
      if (!next) break
      left.delete(next)
      const forward = lower(value(next, 'ID_GEO_Knoten_A')) === at
      const el = geoElement(next)
      at = lower(value(next, forward ? 'ID_GEO_Knoten_B' : 'ID_GEO_Knoten_A'))
      if (el) elements.push(forward ? el : reverseElement(el))
    }
    if (!elements.length || at !== end) {
      report.note('edge_without_geometry', { id, geo: geos.length })
      if (!elements.length) continue
    }
    const sum = elements.reduce((s, el) => s + el.length, 0)
    if (e.length != null && Math.abs(sum - e.length) > 0.05) report.note('edge_length_differs', { id, top: e.length, geo: Math.round(sum * 1000) / 1000 })
    tracks.push(withoutUndefined({
      id, epsg, elements,
      name: names.get(id), trackNumber: names.get(id), lineNumber: lines.get(id),
      status: statusOf(e.o),
    }))
  }
  const trackIds = new Set(tracks.map(t => t.id))

  // ── switches: a node of tip, left and right; the switch element names it ──
  const ends = new Map()   // node → [{ edge, endpoint, conn }]
  for (const [id, e] of edges) {
    if (!trackIds.has(id)) continue
    for (const [node, endpoint, conn] of [[e.a, 'BEGIN', e.connA], [e.b, 'END', e.connB]]) {
      if (!ends.has(node)) ends.set(node, [])
      ends.get(node).push({ edge: id, endpoint, conn })
    }
  }
  /** The node a point object stands at: the end of its edge at Abstand 0 or its length. */
  const nodeAt = (place) => {
    const e = edges.get(place?.edge)
    if (!e || place.abstand == null) return null
    if (Math.abs(place.abstand) < 0.01) return e.a
    if (e.length != null && Math.abs(place.abstand - e.length) < 0.01) return e.b
    return null
  }
  const components = new Map()   // W_Kr_Gsp_Element id → [component]
  const componentOfNode = new Map()
  for (const o of of('W_Kr_Gsp_Komponente')) {
    const element = lower(value(o.el, 'ID_W_Kr_Gsp_Element'))
    const place = placesOf(o.el)[0]
    const node = nodeAt(place)
    const c = { o, element, place, node, trunk: value(o.el, 'Zungenpaar/Stammgleis') }
    if (!components.has(element)) components.set(element, [])
    components.get(element).push(c)
    if (node) componentOfNode.set(node, c)
  }
  const installationOf = (elementId) => get(value(get(elementId)?.el, 'ID_W_Kr_Anlage'))
  const switches = []
  const switchOfNode = new Map()
  for (const [node, list] of ends) {
    const byConn = Object.fromEntries(list.map(x => [x.conn, x]))
    if (list.length === 1) continue
    if (list.length !== 3 || !byConn.Spitze || !byConn.Links || !byConn.Rechts) {
      if (list.length === 2) continue   // two edges end to end: a joint
      report.note('node_not_a_turnout', { id: node, ends: list.map(x => x.conn) })
      continue
    }
    const c = componentOfNode.get(node)
    const element = c ? get(c.element) : null
    const installation = c ? installationOf(c.element) : null
    const art = value(installation?.el, 'W_Kr_Anlage_Allg/W_Kr_Art')
    // The trunk goes on straight: A–B2 is the straight route of a turnout.
    const straight = c?.trunk === 'links' ? byConn.Links : byConn.Rechts
    const branch = straight === byConn.Links ? byConn.Rechts : byConn.Links
    const sw = withoutUndefined({
      switchId: node, kind: SWITCH_KIND[art] ?? 'turnout', formVersion: 1,
      name: element ? (value(element.el, 'Bezeichnung/Bezeichnung_Tabelle') ?? value(element.el, 'Bezeichnung/Bezeichnung_Lageplan_Kurz')) : undefined,
      label: value(installation?.el, 'W_Kr_Anlage_Allg/W_Kr_Grundform'),
      ports: {
        A: { trackId: byConn.Spitze.edge, endpoint: byConn.Spitze.endpoint },
        B1: { trackId: branch.edge, endpoint: branch.endpoint },
        B2: { trackId: straight.edge, endpoint: straight.endpoint },
      },
      status: statusOf(get(node) ?? { start: true, target: true }),
    })
    if (art && !SWITCH_KIND[art]) report.note('switch_kind_as_turnout', { id: node, kind: art })
    switches.push(sw)
    switchOfNode.set(node, sw.switchId)
  }

  // ── end marks: a buffer stop where a Gleis_Abschluss stands, else the planning area's boundary ──
  const endMarks = []
  const closures = new Map()
  for (const o of of('Gleis_Abschluss')) {
    const node = nodeAt(placesOf(o.el)[0]) ?? nearestEnd(placesOf(o.el)[0])
    if (node) closures.set(node, value(o.el, 'Gleis_Abschluss_Art'))
  }
  function nearestEnd(place) {
    const e = edges.get(place?.edge)
    if (!e || place.abstand == null) return null
    const atA = (ends.get(e.a)?.length ?? 0) === 1, atB = (ends.get(e.b)?.length ?? 0) === 1
    if (atA && (!atB || place.abstand < (e.length ?? Infinity) / 2)) return e.a
    return atB ? e.b : null
  }
  for (const [node, list] of ends) {
    if (list.length !== 1) continue
    const kind = closures.get(node)
    endMarks.push(withoutUndefined({
      id: node, kind: BUFFER_STOPS.has(kind) ? 'buffer_stop' : 'boundary',
      trackId: list[0].edge, endpoint: list[0].endpoint,
    }))
  }

  // ── signalling ──
  const loc = (place, { directed = false } = {}) => {
    if (!place || !trackIds.has(place.edge) || place.abstand == null) return null
    const side = SIDE[place.side] ?? (place.lateral < 0 ? 'left' : place.lateral > 0 ? 'right' : undefined)
    return withoutUndefined({
      anchor: 'station', trackId: place.edge, station: place.abstand, side,
      offset: place.lateral != null && place.lateral !== 0 ? Math.abs(place.lateral) : undefined,
      direction: directed || place.wirk ? (place.wirk === 'gegen' ? 'down' : 'up') : undefined,
    })
  }
  const missed = (collection, id, why) => report.note('not_placed', { collection, id, why })
  const name = (el) => value(el, 'Bezeichnung/Bezeichnung_Lageplan_Kurz') ?? value(el, 'Bezeichnung/Oertlicher_Elementname')
  const areaNumber = (el) => value(el, 'Bezeichnung/Kennzahl')

  // Frames and aspects of every signal.
  const framesOf = new Map()   // signal → [{ id, kind, aspects }]
  const frameById = new Map()
  for (const o of of('Signal_Rahmen')) {
    const signal = lower(value(o.el, 'ID_Signal'))
    const kindName = value(o.el, 'Rahmen_Art')
    const frame = { id: idOf(o.el), signal, kind: FRAME[kindName] ?? 'other', aspects: [] }
    if (!FRAME[kindName]) report.note('frame_kind_other', { kind: kindName })
    if (!framesOf.has(signal)) framesOf.set(signal, [])
    framesOf.get(signal).push(frame)
    frameById.set(frame.id, frame)
  }
  const aspectById = new Map()
  for (const o of of('Signal_Signalbegriff')) {
    const frame = frameById.get(lower(value(o.el, 'ID_Signal_Rahmen')))
    const id = child(o.el, 'Signalbegriff_ID')
    const type = id?.attrs['xsi:type']?.split(':').pop()
    if (!frame || !type) continue
    const lit = value(o.el, 'Signal_Signalbegriff_Allg/Beleuchtet')
    const aspect = withoutUndefined({
      aspect: type, symbol: child(id, 'Symbol')?.text || undefined,
      lit: lit === 'ja' ? true : lit === 'nein' ? false : undefined,
      switched: bool(value(o.el, 'Signal_Signalbegriff_Allg/Geschaltet')),
    })
    frame.aspects.push(aspect)
    aspectById.set(idOf(o.el), { signal: frame.signal, aspect })
  }

  const signals = []
  for (const o of of('Signal')) {
    const id = idOf(o.el)
    const places = placesOf(o.el)
    const at = loc(places[0], { directed: true })
    if (!at) { missed('signals', id, 'no_track'); continue }
    const real = child(o.el, 'Signal_Real')
    const fictive = child(o.el, 'Signal_Fiktiv')
    const screen = child(real, 'Signal_Real_Aktiv_Schirm')
    const fn = value(real, 'Signal_Funktion')
    const system = value(screen, 'Signalsystem')
    const s = withoutUndefined({
      id, name: name(o.el), areaNumber: areaNumber(o.el),
      kind: real ? 'real' : 'virtual', at,
      alsoAt: nonEmpty(places.slice(1).map(p => loc(p)).filter(Boolean)),
      bothDirections: places[0].wirk === 'beide' || undefined,
      function: real ? SIGNAL_FUNCTION[fn] : undefined,
      virtualFunction: fictive ? VIRTUAL_FUNCTION[value(fictive, 'Fiktives_Signal_Funktion')] ?? 'other' : undefined,
      logicalSignals: screen ? logicalSignalsOf(SIGNAL_SYSTEM[system], SIGNAL_TYPE[value(screen, 'Signal_Art')]) : undefined,
      mounting: real ? MOUNTING[value(real, 'Signal_Befestigungsart')] : undefined,
      sight: real ? nonEmpty(withoutUndefined({
        required: num(value(real, 'Signalsicht_Soll')), minimum: num(value(real, 'Signalsicht_Mindest')),
        achievable: num(value(real, 'Signalsicht_Erreichbar')),
      })) : undefined,
      frames: real ? (framesOf.get(id) ?? []).map(f => ({ kind: f.kind, aspects: f.aspects })) : undefined,
      lifespan: lifespanOf(o),
    })
    if (fn && real && !SIGNAL_FUNCTION[fn]) report.note('signal_function_unknown', { id, value: fn })
    signals.push(s)
  }
  const signalIds = new Set(signals.map(s => s.id))

  // Train detection.
  const detectionPoints = []
  const boundedBy = new Map()   // FMA_Anlage → [points]
  for (const o of of('FMA_Komponente')) {
    const id = idOf(o.el)
    const at = loc(placesOf(o.el)[0])
    if (!at) { missed('detectionPoints', id, 'no_track'); continue }
    const { direction: _d, ...where } = at
    const art = value(o.el, 'FMA_Komponente_Art')
    detectionPoints.push(withoutUndefined({
      id, name: name(o.el), kind: DETECTION_POINT[art] ?? 'axleCounter', at: where, lifespan: lifespanOf(o),
    }))
    for (const g of childrenNamed(o.el, 'ID_FMAgrenze')) {
      const section = lower(child(g, 'Wert')?.text)
      if (!boundedBy.has(section)) boundedBy.set(section, [])
      boundedBy.get(section).push(id)
    }
  }
  const trackSections = []
  for (const o of of('FMA_Anlage')) {
    const id = idOf(o.el)
    const section = get(value(o.el, 'ID_Gleis_Abschnitt'))
    const letter = value(o.el, 'Bezeichnung/Bezeichnung_Kennbuchstabe') ?? ''
    const short = section ? value(section.el, 'Bezeichnung/Bezeichnung_Lageplan_Kurz') : undefined
    const points = boundedBy.get(id) ?? []
    if (!points.length) { missed('trackSections', id, 'no_points'); continue }
    trackSections.push(withoutUndefined({
      id, name: short != null ? `${letter}${short}` : undefined, areaNumber: section ? areaNumber(section.el) : undefined,
      kind: SECTION_KIND[value(o.el, 'FMA_Anlage_Allg/FMA_Art')] ?? 'other', boundedBy: [...new Set(points)].sort(),
      auxiliaryClearing: bool(value(o.el, 'FMA_Anlage_Allg/FMA_Hilffreimeldung')) || undefined,
      lifespan: lifespanOf(o),
    }))
  }
  const sectionIds = new Set(trackSections.map(t => t.id))
  const sectionRefs = (el, path) => childrenNamed(el, path).map(r => lower(child(r, 'Wert')?.text)).filter(id => sectionIds.has(id))

  // Switch elements, their twofold flank protection.
  const twofold = new Map()
  for (const o of of('Fla_Zwieschutz')) {
    const z = child(o.el, 'Fla_Zwieschutz_Element')
    twofold.set(lower(value(o.el, 'ID_W_Element')), nonEmpty(withoutUndefined({
      kind: TWOFOLD_KIND[value(o.el, 'Zwieschutz_Art')], left: TWOFOLD_MEASURE[value(z, 'Massnahme_L')],
      right: TWOFOLD_MEASURE[value(z, 'Massnahme_R')], preventsRunOn: bool(value(z, 'Nachlaufverhinderung')),
    })))
  }
  const switchElements = []
  for (const o of of('W_Kr_Gsp_Element')) {
    const id = idOf(o.el)
    const comps = components.get(id) ?? []
    const turnout = child(o.el, 'Weiche_Element')
    const derailer = child(o.el, 'Gleissperre_Element')
    const operation = OPERATION[value(o.el, 'W_Kr_Gsp_Element_Allg/W_Kr_Gsp_Stellart')] ?? 'other'
    const base = {
      id, name: name(o.el), areaNumber: areaNumber(o.el), operation,
      preferredAutomatic: bool(value(o.el, 'W_Kr_Gsp_Element_Allg/Vorzugslage_Automatik')),
      lifespan: lifespanOf(o),
    }
    if (derailer) {
      const at = loc(comps[0]?.place, { directed: true })
      if (!at) { missed('switchElements', id, 'no_track'); continue }
      switchElements.push(withoutUndefined({
        ...base, derailer: withoutUndefined({ at, preferredPosition: DERAILER_POSITION[value(derailer, 'Gleissperre_Vorzugslage')] }),
      }))
      continue
    }
    const node = comps.map(c => c.node).find(n => switchOfNode.has(n))
    if (!node) { missed('switchElements', id, 'no_switch'); continue }
    if (comps.filter(c => switchOfNode.has(c.node)).length > 1) report.note('switch_element_many_nodes', { id, nodes: comps.length })
    const marker = lower(value(turnout, 'ID_Grenzzeichen'))
    switchElements.push(withoutUndefined({
      ...base, switchId: switchOfNode.get(node),
      preferredPosition: POSITION[value(turnout, 'Weiche_Vorzugslage')],
      trailable: bool(value(turnout, 'Auffahrortung')),
      clearanceMarkerId: signalIds.has(marker) ? marker : undefined,
      twofoldProtection: twofold.get(id),
    }))
  }
  const elementIds = new Set(switchElements.map(e => e.id))

  // Runs: a Fstr_Fahrweg's parts on edges, in running order from its start's edge.
  const runs = new Map()
  for (const o of of('Fstr_Fahrweg')) {
    const start = lower(value(o.el, 'ID_Start')), target = lower(value(o.el, 'ID_Ziel'))
    const parts = childrenNamed(o.el, 'Bereich_Objekt_Teilbereich').map(p => lower(value(p, 'ID_TOP_Kante')))
    const first = signals.find(s => s.id === start)?.at.trackId
    runs.set(idOf(o.el), {
      start, target, speed: num(value(o.el, 'Fstr_V_Hg')),
      trackIds: inRunningOrder(parts.filter(id => trackIds.has(id)), first, edges),
    })
  }

  // Overlaps.
  const overlaps = []
  const overlapSwitches = new Map()
  for (const o of of('Fstr_DWeg_W_Kr')) {
    const dweg = lower(value(o.el, 'ID_Fstr_DWeg'))
    if (!overlapSwitches.has(dweg)) overlapSwitches.set(dweg, [])
    const el = lower(value(o.el, 'ID_W_Kr_Gsp_Element'))
    if (elementIds.has(el)) overlapSwitches.get(dweg).push({ switchElementId: el, locked: bool(value(o.el, 'Element_Verschluss')) ?? false })
  }
  /**
   * Where a Markanter_Punkt is: the place of the point object it marks — of
   * a clearance marker on two edges, the one on `run` or meeting its last edge.
   */
  const markedPlace = (id, run = []) => {
    const mark = get(id)
    const target = mark && get(value(mark.el, 'ID_Markante_Stelle'))
    const places = target ? placesOf(target.el) : []
    const last = edges.get(run[run.length - 1])
    const meets = (p) => {
      const e = edges.get(p.edge)
      return run.includes(p.edge) || (e && last && [last.a, last.b].some(n => n === e.a || n === e.b))
    }
    return places.find(meets) ?? places[0] ?? null
  }
  for (const o of of('Fstr_DWeg')) {
    const id = idOf(o.el)
    const run = runs.get(lower(value(o.el, 'ID_Fstr_Fahrweg')))
    const endPlace = markedPlace(value(o.el, 'ID_PZB_Gefahrpunkt'), run?.trackIds) ?? (run && markedPlace(run.target, run.trackIds))
    const end = loc(endPlace)
    if (!run || !signalIds.has(run.start) || !end || !run.trackIds.length) { missed('overlaps', id, !end ? 'no_end' : 'no_run'); continue }
    const { direction: _d, side: _s, offset: _o, ...endAt } = end
    overlaps.push(withoutUndefined({
      id, name: value(o.el, 'Bezeichnung/Bezeichnung_Fstr_DWeg'), signalId: run.start, path: { trackIds: run.trackIds },
      end: endAt, length: num(value(o.el, 'Fstr_DWeg_Allg/Laenge_Soll')) ?? 0,
      gradient: num(value(o.el, 'Fstr_DWeg_Allg/Massgebende_Neigung')),
      speed: num(value(o.el, 'Fstr_DWeg_Spezifisch/DWeg_V')),
      releaseDelay: num(value(o.el, 'Fstr_DWeg_Spezifisch/Aufloesung_Verzoegerung')),
      clearSections: nonEmpty(sectionRefs(o.el, 'ID_FMA_Anlage_Freimeldung')),
      switches: nonEmpty(overlapSwitches.get(id)),
      lifespan: lifespanOf(o),
    }))
  }
  const overlapIds = new Set(overlaps.map(x => x.id))

  // Routes and what they show.
  const shown = new Map()
  for (const o of of('Fstr_Signalisierung')) {
    const route = lower(value(o.el, 'ID_Fstr_Zug_Rangier'))
    const a = aspectById.get(lower(value(o.el, 'ID_Signal_Signalbegriff')))
    if (!a || !signalIds.has(a.signal)) continue
    if (!shown.has(route)) shown.set(route, [])
    shown.get(route).push(withoutUndefined({ signalId: a.signal, aspect: a.aspect.aspect, symbol: a.aspect.symbol }))
  }
  const routes = []
  const routeSpeeds = new Map()   // start signal → the speeds it signals per route (Fstr_V_Hg)
  for (const o of of('Fstr_Zug_Rangier')) {
    const id = idOf(o.el)
    const run = runs.get(lower(value(o.el, 'ID_Fstr_Fahrweg')))
    if (!run || !signalIds.has(run.start) || !signalIds.has(run.target)) { missed('routes', id, 'no_signals'); continue }
    const train = child(o.el, 'Fstr_Zug'), shunting = child(o.el, 'Fstr_Rangier'), middle = child(o.el, 'Fstr_Mittel')
    const ovl = childrenNamed(train, 'Fstr_Zug_DWeg')
      .map(d => withoutUndefined({ overlapId: lower(value(d, 'ID_Fstr_DWeg')), preferred: bool(value(d, 'DWeg_Vorzug')) }))
      .filter(d => overlapIds.has(d.overlapId))
    routes.push(withoutUndefined({
      id, kind: train ? 'train' : middle ? 'middle' : 'shunting',
      trainType: known(TRAIN_TYPES, value(train, 'Fstr_Zug_Art')), shuntingType: known(SHUNTING_TYPES, value(shunting, 'Fstr_Rangier_Art')),
      middleType: known(MIDDLE_TYPES, value(middle, 'Fstr_Mittel_Art')),
      rank: num(value(o.el, 'Fstr_Zug_Rangier_Allg/Fstr_Reihenfolge')),
      startSignalId: run.start, targetSignalId: run.target, path: { trackIds: run.trackIds },
      autoSetting: value(train, 'Automatische_Einstellung') === 'ZL' || undefined,
      shortDistantSpacing: bool(value(train, 'Fstr_Vsigabstand_Verkuerzt')) || undefined,
      clearSections: nonEmpty(sectionRefs(shunting, 'ID_FMA_Anlage_Rangier_Frei')),
      overlaps: nonEmpty(ovl),
      aspects: nonEmpty(shown.get(id)),
      lifespan: lifespanOf(o),
    }))
    if (run.speed != null) routeSpeeds.set(run.start, [...(routeSpeeds.get(run.start) ?? []), { routeId: id, speed: run.speed }])
  }
  const routeIds = new Set(routes.map(r => r.id))

  // Flank protection.
  const flankClear = new Map()
  for (const o of of('Fla_Freimelde_Zuordnung')) {
    const f = lower(value(o.el, 'ID_Fla_Schutz'))
    const s = lower(value(o.el, 'ID_FMA_Anlage'))
    if (!sectionIds.has(s)) continue
    if (!flankClear.has(f)) flankClear.set(f, [])
    flankClear.get(f).push(s)
  }
  const flankProtection = []
  for (const o of of('Fla_Schutz')) {
    const id = idOf(o.el)
    const requester = lower(value(o.el, 'Fla_Schutz_Anforderer/ID_Anforderer_Element'))
    const via = VIA[value(o.el, 'Fla_Schutz_Anforderer/Fahrt_Ueber')]
    if (!elementIds.has(requester) || !via) { missed('flankProtection', id, 'requester_not_a_switch'); continue }
    const bySignal = lower(value(o.el, 'Fla_Schutz_Signal/ID_Fla_Signal'))
    const byElement = lower(value(o.el, 'Fla_Schutz_W_Gsp/ID_Fla_W_Gsp_Element'))
    const protection = signalIds.has(bySignal)
      ? withoutUndefined({ signalId: bySignal, targetLock: bool(value(o.el, 'Fla_Schutz_Signal/Fla_Signal_Zielsperrung')) })
      : elementIds.has(byElement)
        ? withoutUndefined({ switchElementId: byElement, position: POSITION[value(o.el, 'Fla_Schutz_W_Gsp/Fla_W_Lage')] })
        : bool(value(o.el, 'Fla_Verzicht')) ? { waived: true } : null
    if (!protection) { missed('flankProtection', id, 'no_protection'); continue }
    flankProtection.push(withoutUndefined({
      id, requester: { switchElementId: requester, via }, protection,
      passOn: nonEmpty(withoutUndefined({
        left: lower(value(o.el, 'Fla_Schutz_Weitergabe/ID_Fla_Weitergabe_L')),
        right: lower(value(o.el, 'Fla_Schutz_Weitergabe/ID_Fla_Weitergabe_R')),
      })),
      clearSections: flankClear.get(id), lifespan: lifespanOf(o),
    }))
  }
  // A pass-on names a flank protection the import has.
  const flankIds = new Set(flankProtection.map(f => f.id))
  for (const f of flankProtection) {
    if (!f.passOn) continue
    for (const side of ['left', 'right']) if (f.passOn[side] && !flankIds.has(f.passOn[side])) delete f.passOn[side]
    if (!Object.keys(f.passOn).length) delete f.passOn
  }

  // PZB, as balises of the system pzb.
  const pzbAssignments = new Map()
  for (const o of of('PZB_Element_Zuordnung')) {
    const bp = child(o.el, 'PZB_Element_Zuordnung_BP')
    pzbAssignments.set(idOf(o.el), {
      signalId: lower(value(bp, 'ID_PZB_Element_Bezugspunkt')),
      effect: PZB_EFFECT[value(bp, 'Wirksamkeit')],
      routes: childrenNamed(o.el, 'PZB_Element_Zuordnung_Fstr').map(r => withoutUndefined({
        routeId: lower(value(r, 'ID_Fstr_Zug_Rangier')), effect: PZB_ROUTE_EFFECT[value(r, 'Wirksamkeit_Fstr')] ?? 'other',
      })).filter(r => routeIds.has(r.routeId)),
    })
  }
  const balises = []
  for (const o of of('PZB_Element')) {
    const id = idOf(o.el)
    const at = loc(placesOf(o.el)[0], { directed: true })
    if (!at) { missed('balises', id, 'no_track'); continue }
    const z = pzbAssignments.get(lower(value(o.el, 'ID_PZB_Element_Zuordnung'))) ?? {}
    const gue = child(o.el, 'PZB_Element_GUE')
    balises.push(withoutUndefined({
      id, at, system: 'pzb', kind: gue ? 'speedCheck' : 'magnet',
      settings: withoutUndefined({
        frequency: PZB_FREQUENCY[value(o.el, 'PZB_Art')] ?? '1000',
        ina: bool(value(o.el, 'PZB_Element_GM/PZB_INA')) || undefined,
        testSpeed: gue ? num(value(gue, 'Pruefgeschwindigkeit')) ?? 0 : undefined,
        measuringLength: gue ? num(value(gue, 'GUE_Messstrecke')) ?? 0 : undefined,
        arrangement: gue ? GUE_ARRANGEMENT[value(gue, 'GUE_Anordnung')] : undefined,
      }),
      signalId: signalIds.has(z.signalId) ? z.signalId : undefined, effect: z.effect ?? 'other',
      routes: nonEmpty(z.routes),
      lifespan: lifespanOf(o),
    }))
  }

  const READ = new Set([
    'Signal', 'Signal_Rahmen', 'Signal_Signalbegriff', 'FMA_Komponente', 'FMA_Anlage', 'Gleis_Abschnitt', 'W_Kr_Gsp_Element',
    'W_Kr_Gsp_Komponente', 'W_Kr_Anlage', 'Fstr_Zug_Rangier', 'Fstr_Fahrweg', 'Fstr_DWeg', 'Fstr_DWeg_W_Kr', 'Fstr_Signalisierung',
    'Fla_Schutz', 'Fla_Freimelde_Zuordnung', 'Fla_Zwieschutz', 'PZB_Element', 'PZB_Element_Zuordnung', 'TOP_Kante', 'TOP_Knoten',
    'GEO_Kante', 'GEO_Knoten', 'GEO_Punkt', 'Gleis_Abschluss', 'Gleis_Bezeichnung', 'Strecke', 'Markanter_Punkt',
  ])
  for (const o of objects.values()) if (!READ.has(o.tag)) report.skip(o.tag)

  const signalling = {
    fields: { catalogue: { ...CATALOGUE_REF } },
    collections: {
      signals: signals.map(s => byLogicalSignal(s, routeSpeeds.get(s.id))), detectionPoints, trackSections, switchElements,
      routes, overlaps, flankProtection, balises, stationBoundaries: [],
    },
  }
  return { tracks, switches, endMarks, signalling, report }
}

/** The places a point object stands at (PlanPro Punkt_Objekt_TOP_Kante), in their order. */
function placesOf(el) {
  return childrenNamed(el, 'Punkt_Objekt_TOP_Kante').map(p => ({
    edge: lower(value(p, 'ID_TOP_Kante')), abstand: num(value(p, 'Abstand')), wirk: value(p, 'Wirkrichtung'),
    side: value(p, 'Seitliche_Lage'), lateral: num(value(p, 'Seitlicher_Abstand')),
  }))
}

/**
 * The edges of a run in the order a train runs them: from `first`, each next
 * the one that shares the node the run left the last by. The parts as they
 * came where they do not chain.
 */
function inRunningOrder(parts, first, edges) {
  const unique = [...new Set(parts)]
  if (unique.length < 2 || !unique.includes(first)) return unique
  const out = [first]
  const left = new Set(unique.filter(id => id !== first))
  let open = null   // the node the run is at, once known
  while (left.size) {
    const last = edges.get(out[out.length - 1])
    const nodes = open ? [open] : [last.a, last.b]
    const next = [...left].find(id => { const e = edges.get(id); return nodes.includes(e.a) || nodes.includes(e.b) })
    if (!next) return unique
    const e = edges.get(next)
    const via = nodes.includes(e.a) ? e.a : e.b
    if (!open && out.length === 1) open = via
    open = via === e.a ? e.b : e.a
    out.push(next)
    left.delete(next)
  }
  return out
}

/**
 * A record of the suite from a PlanPro plan (`text`, the XML): its net as the
 * core's tracks, switches and end marks, its signalling as the domain.
 * `id` and `title` are the record's. Returns { record, report, header } —
 * `report` with what could not be read or placed (`notes`) and the objects
 * of kinds the record does not hold (`skipped`, by tag).
 */
export function planproRecord(text, { id, title } = {}) {
  const model = readPlanPro(text)
  const projectId = id ?? 'planpro'
  const { tracks, switches, endMarks, signalling, report } = convert(model, projectId)
  const record = withoutUndefined({
    id: projectId, title: title ?? model.header.title ?? model.header.subInstallation,
    tracks, switches, platforms: [], endMarks, signalling,
  })
  return { record, report: report.summary(), header: model.header }
}

/**
 * The signalling of a plan placed on another net: every location carried by
 * where it stands onto the track of `onto` that lies there, an object nothing
 * lies under orphaned in the plane (R7) — so a plan's signalling comes onto a
 * project whose tracks are its own and not the plan's edges. `from` is the
 * plan's net (planproRecord's tracks). Returns { signalling, carried }.
 */
export function placeSignalling(signalling, from, onto) {
  const old = new Map(from.map(t => [t.id, t]))
  const ctx = {
    present: new Set(onto.tracks.map(t => t.id)),
    changed: (id) => old.has(id),
    oldTrack: (id) => old.get(id) ?? null,
    candidates: () => onto.tracks,
    tracks: onto.tracks,
    switches: onto.switches ?? [],
  }
  const { domain, carried } = carryDomain(signalling, ctx)
  return { signalling: domain, carried }
}

/**
 * The logical signals of a screen, by its PlanPro Signalsystem and Signal_Art:
 * one of that system; a Sperrsignal of no system is one of the shunting
 * signals' (sh); anything else of no system has none.
 */
function logicalSignalsOf(system, type) {
  if (system) return [withoutUndefined({ system, type })]
  if (type === 'shunting') return [{ system: 'sh', type }]
  return undefined
}

/**
 * A signal as the domain holds it: its function, screen and indicators its
 * logical signal's, with the speeds it signals for the routes from it; its
 * boards and name plate its own. A signal of no system keeps them all.
 */
function byLogicalSignal(signal, speeds) {
  if (!signal.logicalSignals?.length) return signal
  const { function: fn, frames = [], logicalSignals: [first, ...others], ...place } = signal
  const shown = frames.filter(f => f.kind === 'screen' || f.kind === 'indicator')
  return {
    ...place,
    frames: frames.filter(f => !shown.includes(f)),
    logicalSignals: [withoutUndefined({ ...first, function: fn, frames: nonEmpty(shown), signallingSpeeds: nonEmpty(speeds) }), ...others],
  }
}
