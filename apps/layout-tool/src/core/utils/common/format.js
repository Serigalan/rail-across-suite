// Small helpers for numbers that every part of the app used to write itself.
// `clamp` is geometry's, which the computing modules share; offered here too.
export { clamp } from '@rail-across/core/geometry'

/** `v` rounded to `digits` decimals (as a number). */
const roundTo = (v, digits = 3) => Math.round(v * 10 ** digits) / 10 ** digits

/**
 * A number as text: `digits` decimals, a decimal comma when `comma` is set, and
 * with `trim` no trailing zeros (2.500 → 2.5, 3.000 → 3).
 */
export function formatNumber(v, { digits = 3, comma = false, trim = false } = {}) {
  let s = roundTo(v, digits).toFixed(digits)
  if (trim && digits > 0) s = s.replace(/0+$/, '').replace(/\.$/, '')
  return comma ? s.replace('.', ',') : s
}
