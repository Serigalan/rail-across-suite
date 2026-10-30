import { LOGO_WORDS } from './logoWords'

// The marks of the suite: its logo, and one icon per module and tool — line
// drawings in currentColor on a 48 × 48 grid, in the manner of the
// Open-Layout-Tool's sidebar icons.

// The logo: half a rail's profile drawn as one line between two points, the
// rail's axis dashed beside it, and the name. The profile is in the primary
// colour, the name in the text colour (so it reads in the dark as well).
const PROFILE = 'M250 50h-26.6q-16.9 0-16.9 16.9v39.4l24.5 6.5q8.6 2.2 8.6 10.2v75q0 11-10.6 14.5'
  + 'l-56.7 21.2q-3.5 1.3-3.5 4.9v7.7q0 3.7 3.9 3.7H250'

export const Logo = ({ className }) => (
  <svg viewBox="160 40 320 220" className={className} role="img" aria-label="Rail-Across-Suite">
    <path d="M250 50v200" stroke="#888780" strokeDasharray="3 4" opacity=".6" fill="none" vectorEffect="non-scaling-stroke" />
    <path d={PROFILE} fill="none" stroke="var(--color-primary)" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"
      vectorEffect="non-scaling-stroke" />
    <circle cx="250" cy="50" r="5" fill="var(--color-primary)" />
    <circle cx="250" cy="250" r="5" fill="var(--color-primary)" />
    <path d={LOGO_WORDS} fill="currentColor" />
  </svg>
)

const Icon = ({ children }) => (
  <svg viewBox="0 0 48 48" className="ra-icon" fill="none" stroke="currentColor" strokeWidth="2.4"
    strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    {children}
  </svg>
)

const ICONS = {
  // A through track and a branch leaving it.
  infrastructure: (
    <Icon>
      <path d="M4 34 H44" />
      <path d="M10 34 C22 34 24 16 36 16 H44" />
      <circle cx="10" cy="34" r="2.6" fill="currentColor" />
    </Icon>
  ),
  // A railcar from the side.
  vehicles: (
    <Icon>
      <path d="M6 32 V16 a4 4 0 0 1 4 -4 H34 l8 10 V32 Z" />
      <path d="M11 18 H17 V24 H11 Z M22 18 H28 V24 H22 Z M34 18 l4 6" />
      <circle cx="14" cy="36" r="3" />
      <circle cx="34" cy="36" r="3" />
    </Icon>
  ),
  // A graphic timetable: stations across, trains as lines through time.
  timetable: (
    <Icon>
      <path d="M6 8 V40 M42 8 V40 M6 24 H42" strokeOpacity="0.45" />
      <path d="M6 10 L42 26 M6 20 L42 38 M6 40 L42 14" />
    </Icon>
  ),
  // The timetable played through: the graphic timetable's lines behind a play mark.
  timetableSim: (
    <Icon>
      <path d="M6 10 L42 26 M6 24 L42 40" strokeOpacity="0.45" />
      <circle cx="24" cy="24" r="12" />
      <path d="M21 18 V30 L31 24 Z" fill="currentColor" />
    </Icon>
  ),
  // Capacity: a gauge on the line — how far it is filled.
  capacitySim: (
    <Icon>
      <path d="M6 34 a18 18 0 0 1 36 0" />
      <path d="M12 22 l3 2 M24 16 v4 M36 22 l-3 2" />
      <path d="M24 34 L33 23" />
      <circle cx="24" cy="34" r="2.6" fill="currentColor" />
      <path d="M4 42 H44" />
    </Icon>
  ),
  // A construction site on the track: a barrier with its stripes.
  constructionSim: (
    <Icon>
      <path d="M4 40 H44" />
      <path d="M8 18 H40 V26 H8 Z" />
      <path d="M14 18 l-6 8 M24 18 l-6 8 M34 18 l-6 8 M40 20 l-4 6" />
      <path d="M12 26 V40 M36 26 V40" />
    </Icon>
  ),
  // The layout tool: an alignment of straights and an arc, its elements' ends marked.
  layout: (
    <Icon>
      <path d="M4 38 H16 C26 38 30 30 32 22 C34 14 38 10 44 10" />
      <circle cx="16" cy="38" r="2.6" fill="currentColor" />
      <circle cx="32" cy="22" r="2.6" fill="currentColor" />
    </Icon>
  ),
  // The signalling tool: a main signal on its mast.
  signalling: (
    <Icon>
      <path d="M20 44 V26 M14 44 H26" />
      <rect x="13" y="4" width="14" height="22" rx="4" />
      <circle cx="20" cy="11" r="2.6" fill="currentColor" />
      <circle cx="20" cy="19" r="2.6" />
    </Icon>
  ),
  // The catenary tool: a mast, its cantilever and the contact wire.
  catenary: (
    <Icon>
      <path d="M10 44 V6 M6 44 H14" />
      <path d="M10 12 H38 M10 20 L38 14 M30 13 V22" />
      <path d="M4 22 H44" />
    </Icon>
  ),
}

/** The icon of a module or tool, by its id. */
export const ModuleIcon = ({ id }) => ICONS[id] ?? null
