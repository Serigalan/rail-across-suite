// The track module's constants — values the page reads and lists (decision
// 0021, phase D: the functions are the core's Rust module, crates/track; what
// they compute with is stated there again).

/** End marks: a buffer stop and a boundary of the planning area. */
export const BUFFER_STOP = 'buffer_stop'
export const BOUNDARY = 'boundary'

/**
 * The buffer stop types. A placeholder for now (ROADMAP decision 76): the type
 * is stored and proposes the brake length, and does nothing else.
 */
export const BUFFER_STOP_TYPES = [4, 6, 8, 10]
export const DEFAULT_BUFFER_STOP_TYPE = BUFFER_STOP_TYPES[0]

/** Length of the buffer stop itself along the track [m] (decision 77). */
export const BUFFER_STOP_LENGTH = 2.2

/** Half the width of a buffer stop's face and of a boundary's stroke across the track [m]. */
export const FACE_HALF_WIDTH = 1.75
export const BOUNDARY_HALF_WIDTH = 3

/** Heights: a track is split into height pieces past HEIGHT_SPLIT_MIN, points proposed every HEIGHT_POINT_SPACING [m]. */
export const HEIGHT_SPLIT_MIN = 200
export const HEIGHT_POINT_SPACING = 100

/** The vertical datum heights are stated in by default: DHHN2016. */
export const DEFAULT_HEIGHT_EPSG = 7837

/** Vertical datums the heights may be stated in (EPSG codes of the height CRS). */
export const HEIGHT_DATUMS = [
  { epsg: 7837, label: 'DHHN2016' },
  { epsg: 5783, label: 'DHHN92' },
  // Older official datums, as the DB ASCII interface delivers them (mdbGradient).
  { epsg: 7699, label: 'DHHN12' },
  { epsg: 5784, label: 'DHHN85' },
  { epsg: 5785, label: 'SNN76 (HN 76)' },
  { epsg: 5773, label: 'EGM96' },
  { epsg: 3855, label: 'EGM2008' },
]

/**
 * The two values of a gradient point that are given when it is edited in the
 * Höhenplan table — the other two follow, its neighbours stay (solveHeightPoint):
 * s station, z height, gb / ga the gradient before / after it.
 */
export const GIVEN_MODES = {
  sz: ['s', 'z'],
  sgb: ['s', 'gb'],
  sga: ['s', 'ga'],
  zgb: ['z', 'gb'],
  zga: ['z', 'ga'],
  gbga: ['gb', 'ga'],
}

/** The codes of the track ids' type and side parts, and their names. */
export const TYPE_CODES = { line_track: 1n, station_track: 2n }
export const SIDE_CODES = { sorting: 1n, non_sorting: 2n }
/** A track's kind and side as the record states them (core 4.0, decision 0029, phase 6), and the names a form and oltjson give them. */
export const TYPE_NAMES = { line: 'line_track', station: 'station_track' }
export const SIDE_NAMES = { regular: 'sorting', opposite: 'non_sorting' }
export const TRACK_TYPE_OF = { line_track: 'line', station_track: 'station' }
export const TRACK_SIDE_OF = { sorting: 'regular', non_sorting: 'opposite' }
export const LINE_CATEGORIES = ['main', 'secondary', 's_bahn']
export const TRACK_USES = ['main', 'siding']

/** A track ends as close as this to another to be joined to it [m]. */
export const JOINT_TOL = 1.5

/** The kinds of switch record: a turnout by default, a link joining two tracks. */
export const DEFAULT_SWITCH_KIND = 'turnout'
export const LINK_KIND = 'link'
/** A construction stage connector (decision 0027): A joined to B1 while that exists and to B2 from then on — no switch. */
export const STAGE_LINK_KIND = 'stage_link'
export const SWITCH_KINDS = ['turnout', 'crossing', 'single_slip', 'double_slip', LINK_KIND, STAGE_LINK_KIND]
/** The version of the switch forms a switch record is built on. */
export const SWITCH_FORM_VERSION = 1
/** A turnout's routes and its ports with the route each carries (switchRoutes, switchPorts of a turnout). */
export const SWITCH_ROUTES = ['main', 'branch']
export const SWITCH_PORTS = [{ port: 'A', route: 'main' }, { port: 'B1', route: 'branch' }, { port: 'B2', route: 'main' }]

/** The planning status and the colour a plan draws it in. */
export const STATUSES = ['existing', 'new', 'removal']
export const STATUS_COLOR = { existing: '#000000', new: '#ff0000', removal: '#e6b400' }

/** Platforms: the edge's distance from the track axis [m], the heights over top of rail [mm], the code's length. */
export const PLATFORM_FRONT_OFFSET = 1.68
export const PLATFORM_WIDTH = 3.00
export const PLATFORM_BACK_OFFSET = PLATFORM_FRONT_OFFSET + PLATFORM_WIDTH
export const PLATFORM_HEIGHTS = [380, 550, 760]
export const DEFAULT_PLATFORM_HEIGHT = 550
export const PLATFORM_CODE_MAX = 4

/** The platform dialog's fields before anything is entered. */
export const EMPTY_PLATFORM_FORM = Object.freeze({
  trackId: null, start: '', end: '', side: 'right',
  height: DEFAULT_PLATFORM_HEIGHT,     // mm over top of rail
  freeHeight: false,                   // a height outside the standard ones
  stationName: '', code: '',
  // The edge distance is proposed, not fixed: one value for every height and
  // cant at this design stage, and overridable where the site demands it.
  frontOffset: PLATFORM_FRONT_OFFSET,
})
