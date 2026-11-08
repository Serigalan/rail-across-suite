// The one tolerance lengths are compared with when the question is whether two
// computed places are the same place — a node, a toe on an element's end, a
// piece too short to be one. Every module asks it from here, so they all mean
// the same by "the same place", and so will the Rust crates that take over.
//
// Not here, on purpose: a solver's stopping criterion (finer than this), the
// tolerances an import matches foreign data with (that data is mm or cm
// precise), the rules' own distances, and anything in another unit (degrees,
// mm of cant, pixels, heights).

/** Decimals positions, lengths, radii and stations are kept to: 0.1 µm (dehydrateProjects). */
export const POSITION_DIGITS = 7

/**
 * Two points or two stations closer than this are the same place [m]: 1 µm.
 * It stands above what keeping positions to 0.1 µm can move two points apart
 * (√2 × 0.1 µm), so a chain that joined still joins when read back.
 */
export const NODE_TOL = 1e-6
