// The cross section's constants and the catalogue of Ril 800.0130 as the page
// lists it (decision 0005: the functions are the core's Rust module,
// crates/track's cross section, which reads the same catalogue).

import CROSS_SECTION_CATALOGUE from '../../constraints/db-ril-800-0130.json' with { type: 'json' }

export { CROSS_SECTION_CATALOGUE }

const A01 = CROSS_SECTION_CATALOGUE.sources.find(s => s.id === CROSS_SECTION_CATALOGUE.lichtraum.source.ref)
// "DB Ril 800.0130A01 — Lichtraum" → "DB Ril 800.0130A01": a reference, not a
// name to translate — the same way a Ril's number is stated everywhere else.
export const CLEARANCE_SOURCE = A01.title.split(' — ')[0]

export const GAUGE_PROFILES = Object.fromEntries(
  CROSS_SECTION_CATALOGUE.lichtraum.profile.map(p => [p.id, {
    id: p.id,
    points: p.umriss,
    einragungen: p.einragungen ?? [],
    areaKinds: p.einragungen_arten ?? [],
  }]))

// A project saved before the Ril's profiles came in names 'en15273_gc', which
// is gone; it falls back to this one, the profile that one approximated.
export const DEFAULT_GAUGE_PROFILE = 'hauptgleis'

/** Distance between the two running circles, and between the rail inner faces [mm]. */
export const RUNNING_CIRCLE_DISTANCE = 1500
export const TRACK_GAUGE             = 1435

/**
 * Rail profiles by their current designation, the former name alongside because
 * that is what drawings and the MDB stock data still say (Entscheidung 131).
 * All in mm, after DIN EN 13674-1 (figures from the Dlubal catalogue, which
 * lists the profiles under their former names).
 *
 * `head` is the head's width at the top, `headBottom` where its flanks end —
 * they spread 1:20. Heights are measured from the head's top: `height` the
 * whole rail, `headHeight` down to the web, `flankHeight` the head's vertical
 * flank. A worn rail is lower by up to about 10 mm; the `…Worn` figures are the
 * least a rail in service may have, so a head height of a 54 E4 lies between
 * 44.2 and 55 mm.
 */
export const RAILS = {
  '49E5': {
    label: '49 E5 (S 49)', head: 67, headBottom: 70, web: 14, foot: 125,
    height: 149, heightWorn: 139.1, headHeight: 51.5, headHeightWorn: 41.6, flankHeight: 39.8, flankHeightWorn: 29.8,
  },
  '54E4': {
    label: '54 E4 (S 54)', head: 67, headBottom: 70, web: 16, foot: 125,
    height: 154, heightWorn: 143.2, headHeight: 55, headHeightWorn: 44.2, flankHeight: 43.3, flankHeightWorn: 32.5,
  },
  '60E2': {
    label: '60 E2 (UIC 60)', head: 72, headBottom: 74.3, web: 16.5, foot: 150,
    height: 172, heightWorn: 162.6, headHeight: 51, headHeightWorn: 41.6, flankHeight: 37.5, flankHeightWorn: 28.1,
  },
}

/** Sleepers by type, with what each is laid for (`use` is a translation key). */
export const SLEEPERS = {
  B70:    { label: 'B70',             length: 2600, width: 300, height: 210, use: 'normal' },
  B90:    { label: 'B90',             length: 2600, width: 320, height: 200, use: 'switch_zone' },
  SWITCH: { label: 'Weichenschwelle', length: 2600, width: 300, height: 220, use: 'switch' },
}

/** What a track is built with where it says nothing else. */
export const DEFAULT_RAIL    = '54E4'
export const DEFAULT_SLEEPER = 'B70'


/** Station a track's element begins at [m]. */

/**
 * Planum edge beyond the outermost track [mm]: the widest the Regelquerschnitte
 * of Ril 800.0130 ask for. The section is fitted to reach at least that far, so
 * the ground beside the track is in view and not only the track itself.
 */
export const FORMATION_EDGE = 1000 * Math.max(
  ...CROSS_SECTION_CATALOGUE.streckenquerschnitte.rows.map(r => r.planumskante))

/** The chosen profile, or the default where a project names one that is gone. */
export const gaugeProfile = (key) => GAUGE_PROFILES[key] ?? GAUGE_PROFILES[DEFAULT_GAUGE_PROFILE]

/** Locale key naming a profile — the Ril's own words for what it is for. */
export const gaugeProfileLabelKey = (id) => `constraints_querschnitt_profil_${id}`

/** The top of rail of a track without a gradient, over the ground at its axis [m]. */
export const ASSUMED_RAIL_OVER_TERRAIN = 0.20
