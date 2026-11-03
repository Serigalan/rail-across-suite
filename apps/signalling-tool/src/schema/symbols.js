import { hasScreen } from '../model/domain.js'

// What the schematic view draws an object as, and how long a symbol is —
// the shapes themselves are planzeichen.jsx's.

/** Which signalling systems' kinds draw as which screen. */
const FAMILY = {
  multiSection: 'multi', multiSectionShunting: 'multi', main: 'main', mainShunting: 'main',
  distant: 'distant', distantRepeater: 'distant', shunting: 'shunting',
}
const RANK = { multi: 4, main: 3, distant: 2, shunting: 1 }

/** The boards by the aspect they show: shape, and the text in it. */
const BOARDS = {
  Ne_1: { shape: 'trapezoid' },
  Ne_2: { shape: 'cross' }, Ne_2_vRVA: { shape: 'cross' },
  Ne_4: { shape: 'checker' },
  Ne_5: { shape: 'halt' },
  Lf_1: { shape: 'triangle' }, Lf_1_Wdh: { shape: 'triangle' }, Lf_4_DS: { shape: 'triangle' }, Lf_4_DV: { shape: 'triangle' },
  Lf_6: { shape: 'triangle' },
  Lf_1_2: { shape: 'square' }, Lf_7: { shape: 'square' }, Zs_3: { shape: 'square' }, Zs_3v: { shape: 'square' },
  Lf_2: { shape: 'square', text: 'A' }, Lf_3: { shape: 'square', text: 'E' },
  Ne_14: { shape: 'etcsStop' },
}

/** Mountings a board hangs from. */
const HANGING = new Set(['bracket', 'gantry'])

/**
 * What a signal is drawn as: { kind: 'virtual' }, { kind: 'board', shape,
 * text, hanging } — hanging from a bracket or gantry —, or { kind: 'signal',
 * family, shunt, open } — the strongest screen of its logical signals, a
 * shunting signal on the same mast, and whether its screen is drawn open. The
 * plans draw a screen filled or open by who works the signal; the domain does
 * not say that yet, so every screen is filled.
 */
export function signalSymbolOf(signal) {
  if (signal.kind === 'virtual') return { kind: 'virtual' }
  if (!hasScreen(signal)) {
    const aspects = (signal.frames ?? []).flatMap(f => f.aspects ?? [])
    const shown = aspects.find(a => BOARDS[a.aspect]) ?? aspects[0]
    if (!shown) return { kind: 'board', shape: 'plate', text: '?' }
    const board = BOARDS[shown.aspect]
    if (!board) return { kind: 'board', shape: 'plate', text: shown.aspect.replace(/_/g, ' ') }
    return { kind: 'board', shape: board.shape, text: board.text ?? shown.symbol ?? '', hanging: HANGING.has(signal.mounting) }
  }
  const logicals = signal.logicalSignals ?? []
  const families = logicals.map(l => FAMILY[l.type]).filter(Boolean)
  const family = families.sort((a, b) => RANK[b] - RANK[a])[0] ?? 'main'
  const shunt = family !== 'shunting'
    && logicals.some(l => l.type === 'mainShunting' || l.type === 'multiSectionShunting' || l.type === 'shunting')
  return { kind: 'signal', family, shunt, open: false }
}

export const MAST = { multi: 14, main: 14, distant: 14, shunting: 9 }

/**
 * A signal's screen at the end of its mast, its outline and where it ends: a
 * main signal's rounded away from the mast, a multi-section signal's at both
 * ends, a distant signal's towards the mast.
 */
export function screen(family, L) {
  switch (family) {
    case 'multi': return { d: `M${L + 5},-5 H${L + 15} A5,5 0 0 1 ${L + 15},5 H${L + 5} A5,5 0 0 1 ${L + 5},-5 Z`, end: L + 20 }
    case 'distant': return { d: `M${L + 5},-5 H${L + 9} V5 H${L + 5} A5,5 0 0 1 ${L + 5},-5 Z`, end: L + 9 }
    default: return { d: `M${L},-5 H${L + 6} A5,5 0 0 1 ${L + 6},5 H${L} Z`, end: L + 11 }
  }
}

/** Where a symbol ends along its mast, for its name beyond it. */
export function symbolLength(symbol) {
  if (symbol.kind === 'virtual') return 8
  if (symbol.kind === 'board') return 22
  if (symbol.family === 'shunting') return MAST.shunting + 9
  return screen(symbol.family, MAST[symbol.family]).end
}
