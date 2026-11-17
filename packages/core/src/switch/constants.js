// The switch module's catalogue as the page lists it — the forms of DB Ril
// 800.0120 (`constraints/db-ril-800-0120.json`) by kind and class — and its
// constants (decision 0005: the functions are the core's Rust module,
// crates/switch, which reads the same catalogue).
import CATALOGUE from '../../constraints/db-ril-800-0120.json' with { type: 'json' }

export { CATALOGUE as SWITCH_CATALOGUE }

const FORMS = CATALOGUE.formen

/**
 * Any form — turnout or crossing, Regelform or Sonderbauform — looked up by its
 * label. The label is the key a saved project keeps, which is why the
 * catalogue states outright that a label is never renamed.
 */
export const switchTypeByLabel = (label) => FORMS.find(form => form.label === label) ?? null
const turnouts = (klasse) => FORMS.filter(form => form.art === 'weiche' && form.klasse === klasse)

/** The standard turnouts (Regelweichen), as the pickers offer them first. */
export const SWITCH_TYPES = turnouts('regel')
/** The special forms (Sonderbauformen). */
export const SWITCH_TYPES_SPECIAL = turnouts('sonderbauform')
/** Every form a picker offers: the standard ones, then the special ones. */
export const SWITCH_PICK_TYPES = [...SWITCH_TYPES, ...SWITCH_TYPES_SPECIAL]

/**
 * What a picker starts on: `300 – 1:9`, the form the dialogs have opened with
 * since they had one table. Found by name, not by position — AP 3.1 dropped a
 * form and shifted every index behind it once already.
 */
export const DEFAULT_SWITCH_TYPE_IDX = SWITCH_PICK_TYPES.findIndex(form => form.label === '300 – 1:9')

/**
 * The stages a switch connection falls back through, and the order within
 * each: `verbindung` on the form, which the catalogue marks as this tool's own
 * decision rather than the Ril's.
 */
const stage = (n) => FORMS
  .filter(form => form.verbindung?.stufe === n)
  .sort((a, b) => a.verbindung.rang - b.verbindung.rang)
export const SWITCH_CONNECTION_STAGES = [stage(1), stage(2), stage(3)]

/** Every turnout form, whatever its class — what an import matches a record against. */
export const ALL_SWITCH_TYPES = FORMS.filter(form => form.art === 'weiche')
/** The crossing forms. */
export const CROSSING_TYPES = FORMS.filter(form => form.art === 'kreuzung')

/** The catalogue's own description: id, title, version. */
export const SWITCH_RULEBOOK = CATALOGUE.katalog
export const SWITCH_RULEBOOK_ID = SWITCH_RULEBOOK.id

/** A height point this close to the ldS is the ldS point [m]; heights that agree to Z_TOL agree (kept to 0.1 mm). */
export const STATION_TOL = 0.01
export const Z_TOL = 0.00005

/** How many elements the connecting chain to an open end may have, at the most; how many of the middle chain's values may be left open. */
export const END_CHAIN_OPEN = 3
export const MID_OPEN = 2

/** The speeds a switch connection is laid for, those of its first stage's forms. */
export const CONNECTION_SPEEDS = [...new Set(SWITCH_CONNECTION_STAGES[0].map(type => type.speed))]

/** The detail drawing: the rails' distance from the axis, how far a sleeper reaches beyond the outer rail [m]. */
export const RAIL_OFFSET = 0.75
export const SLEEPER_OVERHANG = 0.55
/** The sleepers: the first one behind WA, and their spacing behind WE [m]. */
export const FIRST_SLEEPER = 0.3
export const SLEEPER_SPACING = 0.6
/** A curvature this small is straight [1/m]. */
export const STRAIGHT_CURVATURE = 1e-7
