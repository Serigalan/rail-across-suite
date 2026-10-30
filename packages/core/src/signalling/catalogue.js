import KATALOG from '../../constraints/db-ril-301.json' with { type: 'json' }

// The signal aspects of DB Ril 301 and what each kind of signal shows — the
// content the signalling schema leaves open. The schema says a frame holds
// aspects by key; packages/core/constraints/db-ril-301.json says which keys
// there are, on which frame, with which symbol, and which a Ks main signal, a
// Ks distant signal or a shunting signal has (decision 0011). The keys are
// PlanPro's names of the aspects (Hp_0, Ks_1, Zs_3 …), so an export needs no
// table.

/** The catalogue itself, for a viewer or a test that wants more. */
export { KATALOG as SIGNAL_CATALOGUE }

/** The catalogue as the signalling domain names it in its fields. */
export const CATALOGUE_REF = { id: KATALOG.katalog.id, version: KATALOG.katalog.katalog_version }

const ASPECTS = new Map(KATALOG.begriffe.map(b => [b.id, b]))

/** The aspect of that key, or undefined. */
export const aspectOf = (key, catalogue = KATALOG) => (catalogue === KATALOG ? ASPECTS.get(key) : catalogue.begriffe.find(b => b.id === key))

/**
 * What a kind of signal shows, by its system (a key of `systeme`, or none):
 * { title, schirm: { pflicht, erlaubt }, zusatz }, or undefined.
 */
export function signalKindOf(system, type, catalogue = KATALOG) {
  const kinds = system ? catalogue.systeme[system]?.signalarten : catalogue.ohne_system.signalarten
  return kinds?.[type]
}

/** Whether `system` is a signalling system of the catalogue. */
export const knownSystem = (system, catalogue = KATALOG) => Object.hasOwn(catalogue.systeme, system)

/** Whether an aspect may stand on a plate of its own (a board, a Zs 3 as a plate). */
export const boardAspect = (key, catalogue = KATALOG) => catalogue.ohne_system.tafeln.includes(key)

/** Whether `symbol` is what the aspect asks for: none where it has no symbol, else one that matches its pattern. */
export function symbolFits(aspect, symbol, catalogue = KATALOG) {
  if (!aspect.symbol) return symbol === undefined
  if (symbol === undefined) return false
  return new RegExp(catalogue.symbole[aspect.symbol].pattern).test(symbol)
}
