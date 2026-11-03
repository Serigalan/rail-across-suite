/**
 * The regelwerk values the app needs synchronously — cant limits a dialog
 * judges on every keystroke, before any fetch could return.
 *
 * They are no longer a hand-kept copy of anything. Since AP R.8 they are read
 * out of the rulebook itself: `regelkatalog.js` bundles DB Ril 800.0110 and
 * `catalogLimit` evaluates the very rule a dialog is held to. The drift the
 * AP R.3 note was about — two independently maintained copies with no test
 * between them — is gone by construction. The optimizer has no copy either:
 * the app hands its module the very file it bundles (decision 0019 of the
 * suite), so the two halves of the one Ril cannot part company at all.
 */

import { catalogLimit, IN_SWITCH_AREA } from './regelkatalog.js'

/** The regelwerk a run uses unless it is told otherwise — DB Ril 800.0110. */
export const REGELWERK_ID = 'db-ril-800-0110'

// LP.KB.05 — the cant a turnout may carry (weiche.u_max in the served copy).
export const MAX_SWITCH_CANT = catalogLimit('LP.KB.05', 'reg', { 'element.cant': 0 }, IN_SWITCH_AREA)

// LP.KB.06 — the deficiency a turnout may carry (weiche.uf_max).
export const MAX_SWITCH_CANT_DEF = catalogLimit('LP.KB.06', 'max', { 'physics.u_f': 0 }, IN_SWITCH_AREA)

// packages/core/constraints/physics.json, ueberhoehungsfehlbetrag_koeffizient.wert — v = sqrt(R · (u+uf) / coeff).
// Physics, not a regelwerk: no Ril sets it, it follows from the gauge and g.
export const CANT_DEFICIENCY_COEFF = 11.8
