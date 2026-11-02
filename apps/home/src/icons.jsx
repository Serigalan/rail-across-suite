import { LOGO_WORDS } from './logoWords'

// The marks of the suite: its logo, and one mark per module and tool — a line
// drawing in white on a rounded square of currentColor, 48 × 48, as an app's
// icon. The Fahrzeug-Editor, the Fahrplan-Editor and the Simulation-Tool show
// the same mark at the top of their rails and as their pages' icons.

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

// A module's mark, as an app's: its drawing in white on a rounded square of
// the current colour — the primary colour, grey for what is still to come.
const Icon = ({ children }) => (
  <svg viewBox="0 0 48 48" className="ra-icon" aria-hidden="true">
    <rect width="48" height="48" rx="12" fill="currentColor" />
    <g fill="none" stroke="#fff" strokeWidth="2.6" strokeLinecap="round" strokeLinejoin="round">{children}</g>
  </svg>
)
const Dot = ({ x, y, r = 2.8 }) => <circle cx={x} cy={y} r={r} fill="#fff" stroke="none" />

const ICONS = {
  // A through track and a branch leaving it at the switch.
  infrastructure: (
    <Icon>
      <path d="M8 32 H40" />
      <path d="M14 32 C23 32 25 17 34 17 H40" />
      <Dot x={14} y={32} />
    </Icon>
  ),
  // A multiple unit from the side: its nose, two windows, its bogies on the rail.
  vehicles: (
    <Icon>
      <path d="M8 31 V19 a4 4 0 0 1 4 -4 H31 c2.4 0 4.2 1 5.4 3 L40 25 V31 Z" />
      <path d="M13 19.5 H18.5 V24.5 H13 Z M22 19.5 H27.5 V24.5 H22 Z M31 19.5 H33.6 L36.4 24.5 H31 Z" fill="#fff" stroke="none" />
      <path d="M7 38 H41" strokeOpacity="0.6" />
      <Dot x={15} y={34.5} r={2.4} />
      <Dot x={33} y={34.5} r={2.4} />
    </Icon>
  ),
  // The graphical timetable: two stations across, trains as lines through time.
  timetable: (
    <Icon>
      <path d="M12 9 V39 M36 9 V39" strokeOpacity="0.5" strokeWidth="2" />
      <path d="M12 12 L36 22 M12 24 L36 34 M12 37 L36 15" />
    </Icon>
  ),
  // A run over the distance: speeding up, running, braking, on its axes.
  timetableSim: (
    <Icon>
      <path d="M10 11 V37 H39" strokeOpacity="0.5" strokeWidth="2" />
      <path d="M12 36 C14 24 17 18 22 18 H29 C32 18 34 25 36 36" />
      <path d="M10 18 H14" strokeOpacity="0.5" strokeWidth="2" />
    </Icon>
  ),
  // Capacity: a gauge on the line — how far it is filled.
  capacitySim: (
    <Icon>
      <path d="M10 32 a14 14 0 0 1 28 0" />
      <path d="M14.5 22 l2.4 1.8 M24 17.5 v3 M33.5 22 l-2.4 1.8" strokeWidth="2" />
      <path d="M24 32 L31.5 23.5" />
      <Dot x={24} y={32} />
      <path d="M8 38 H40" strokeOpacity="0.6" />
    </Icon>
  ),
  // A construction site on the track: a barrier with its stripes.
  constructionSim: (
    <Icon>
      <path d="M9 17 H39 V25 H9 Z" />
      <path d="M15 17 l-5 8 M23 17 l-5 8 M31 17 l-5 8 M39 17 l-5 8" strokeWidth="2" />
      <path d="M14 25 V37 M34 25 V37" />
      <path d="M8 38 H40" strokeOpacity="0.6" />
    </Icon>
  ),
  // The Layout-Tool: an alignment of a straight, a transition and an arc, its elements' ends marked.
  layout: (
    <Icon>
      <path d="M8 34 H18 C26 34 28 28 30 22 C32 15 35 13 40 13" />
      <Dot x={18} y={34} />
      <Dot x={30} y={22} />
    </Icon>
  ),
  // The Signalling-Tool: a main signal on its mast.
  signalling: (
    <Icon>
      <path d="M24 40 V28 M18 40 H30" />
      <rect x="17" y="8" width="14" height="20" rx="4" />
      <Dot x={24} y={14} r={2.6} />
      <circle cx="24" cy="21.5" r="2.4" strokeWidth="2" />
    </Icon>
  ),
  // The Catenary-Tool: a mast, its cantilever and the contact wire.
  catenary: (
    <Icon>
      <path d="M13 40 V9 M9 40 H17" />
      <path d="M13 14 H37 M13 21 L37 16 M31 15 V25" strokeWidth="2.2" />
      <path d="M8 25 H40" />
    </Icon>
  ),
}

/** The icon of a module or tool, by its id. */
export const ModuleIcon = ({ id }) => ICONS[id] ?? null
