// Small helpers for numbers that the computing modules share.

/** `v` held between `lo` and `hi`. */
export const clamp = (v, lo, hi) => Math.min(hi, Math.max(lo, v))
