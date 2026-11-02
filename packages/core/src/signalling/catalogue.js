import KATALOG from '../../constraints/db-ril-301.json' with { type: 'json' }

// The signal aspects of DB Ril 301, the signalling systems and what each kind
// of signal shows in them, the train protection systems with their balises
// and magnets — the content the signalling schema leaves open. The schema says
// a frame holds aspects by key, a signal has logical signals of systems, a
// balise a system and kind with settings; packages/core/constraints/db-ril-301.json
// says which keys there are (decision 0011). The keys of the aspects are
// PlanPro's names (Hp_0, Ks_1, Zs_3 …), so an export needs no table. Every
// entry has a German `title` and an English `title_en`.

/** The catalogue itself, for a viewer or a test that wants more. */
export { KATALOG as SIGNAL_CATALOGUE }

/** The catalogue as the signalling domain names it in its fields. */
export const CATALOGUE_REF = { id: KATALOG.katalog.id, version: KATALOG.katalog.katalog_version }

const ASPECTS = new Map(KATALOG.begriffe.map(b => [b.id, b]))

/** The aspect of that key, or undefined. */
export const aspectOf = (key, catalogue = KATALOG) => (catalogue === KATALOG ? ASPECTS.get(key) : catalogue.begriffe.find(b => b.id === key))

/** An entry's title in a language: `title_en` in English, `title` (German) otherwise and where there is none. */
export const titleIn = (entry, language) => (entry ? (language === 'de' ? entry.title : entry[`title_${language}`] ?? entry.title) : undefined)

/**
 * What a kind of signal shows in a system (a key of `systeme`):
 * { title, title_en, schirm: { pflicht, fahrt, erlaubt }, zusatz }, or undefined.
 */
export const signalKindOf = (system, type, catalogue = KATALOG) =>
  (system && Object.hasOwn(catalogue.systeme, system) ? catalogue.systeme[system].signalarten[type] : undefined)

/** Whether `system` is a signalling system of the catalogue. */
export const knownSystem = (system, catalogue = KATALOG) => Object.hasOwn(catalogue.systeme, system)

/**
 * What a signal's screen must and may show, and which indicators it carries:
 * the union over the kinds of its logical signals, as one signal shows the
 * aspects of every system it is part of. { kinds, pflicht, fahrt, erlaubt,
 * zusatz } — `fahrt` per kind, of which each needs one shown; null where no
 * logical signal has a kind of the catalogue.
 */
export function screenOf(logicalSignals, catalogue = KATALOG) {
  const kinds = (logicalSignals ?? []).map(l => signalKindOf(l.system, l.type, catalogue)).filter(Boolean)
  if (!kinds.length) return null
  const union = (pick) => [...new Set(kinds.flatMap(pick))]
  return {
    kinds,
    pflicht: union(k => k.schirm.pflicht),
    fahrt: kinds.map(k => k.schirm.fahrt ?? []).filter(f => f.length),
    erlaubt: union(k => k.schirm.erlaubt),
    zusatz: union(k => k.zusatz),
  }
}

/** A train protection system of the catalogue (`zugbeeinflussung.systeme`), or undefined. */
export const protectionSystemOf = (system, catalogue = KATALOG) =>
  (Object.hasOwn(catalogue.zugbeeinflussung.systeme, system) ? catalogue.zugbeeinflussung.systeme[system] : undefined)

/** A kind of balise or magnet of a train protection system: { title, title_en, pflicht, erlaubt }, or undefined. */
export const baliseKindOf = (system, kind, catalogue = KATALOG) => {
  const arten = protectionSystemOf(system, catalogue)?.arten
  return arten && Object.hasOwn(arten, kind) ? arten[kind] : undefined
}

/**
 * Whether a value fits a setting the catalogue declares: one of its `werte`
 * (auswahl), a number within `min` and `max` (zahl), a boolean
 * (wahrheitswert), a string (text).
 */
export function settingFits(setting, value) {
  switch (setting.typ) {
    case 'auswahl': return typeof value === 'string' && Object.hasOwn(setting.werte, value)
    case 'zahl': return typeof value === 'number' && Number.isFinite(value) && (setting.min == null || value >= setting.min) && (setting.max == null || value <= setting.max)
    case 'wahrheitswert': return typeof value === 'boolean'
    case 'text': return typeof value === 'string' && value !== ''
    default: return false
  }
}

/**
 * How the settings of an object stand to what its system declares (`declared`,
 * name → setting) and its kind asks (`pflicht`) and allows (`erlaubt`):
 * [{ problem: 'missing' | 'unknown' | 'invalid', setting }].
 */
export function settingsProblems(settings, declared, { pflicht = [], erlaubt } = {}) {
  const problems = []
  const given = settings ?? {}
  for (const name of pflicht) if (given[name] === undefined) problems.push({ problem: 'missing', setting: name })
  for (const [name, value] of Object.entries(given)) {
    const setting = declared?.[name]
    if (!setting || (erlaubt && !erlaubt.includes(name))) problems.push({ problem: 'unknown', setting: name })
    else if (!settingFits(setting, value)) problems.push({ problem: 'invalid', setting: name })
  }
  return problems
}

/** Whether an aspect may stand on a plate of its own (a board, a Zs 3 as a plate). */
export const boardAspect = (key, catalogue = KATALOG) => catalogue.ohne_system.tafeln.includes(key)

/**
 * Whether `symbol` is what the aspect asks for: none where it has no symbol,
 * any where its symbol is free text, else one of those the catalogue lists.
 */
export function symbolFits(aspect, symbol) {
  if (!aspect.symbole && !aspect.symbol_frei) return symbol === undefined
  if (symbol === undefined) return false
  return aspect.symbol_frei === true || aspect.symbole.includes(symbol)
}

/** Whether the aspect may stand on a frame of that kind: where the catalogue names none, on any. */
export const aspectFitsFrame = (aspect, frame) => !aspect.rahmen || aspect.rahmen.includes(frame)

/** The speed an aspect's symbol shows [km/h] — Zs 3, Zs 3v, Lf 6, Lf 7 show it in tens — or null. */
export function aspectSpeed(aspect, symbol) {
  if (aspect?.symbol_bedeutung !== 'speed') return null
  const n = Number(String(symbol ?? '').replace(',', '.'))
  return Number.isFinite(n) ? n * 10 : null
}
