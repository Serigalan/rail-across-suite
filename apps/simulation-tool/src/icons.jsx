// The Simulation-Tool's own symbols, on the suite's 24-unit grid in the
// current colour (the rail draws them white).

const Svg = ({ children }) => (
  <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8"
    strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" focusable="false">
    {children}
  </svg>
)

/**
 * The mark of the Fahrplan-Sim, as the start page draws it — the graphical
 * timetable, each kind of train in its own tone — but for the rail's 28 px:
 * two stations, three trains, thick lines.
 */
export const LogoIcon = () => (
  <svg viewBox="0 0 32 32" width="28" height="28" aria-hidden="true">
    <g fill="none" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M6 3.5V28.5M26 3.5V28.5" stroke="#303383" strokeOpacity=".4" strokeWidth="1.6" />
      <path d="M6 5L26 15" stroke="#303383" />
      <path d="M6 15L26 25" stroke="#7462d4" />
      <path d="M26 5.5L6 26.5" stroke="#3d82d1" />
    </g>
  </svg>
)
/** A train: its front, two windows, its wheels on the rail. */
export const TrainIcon = () => <Svg><rect x="5" y="3" width="14" height="14" rx="3" /><path d="M5 10h14" /><circle cx="9" cy="14" r="0.8" /><circle cx="15" cy="14" r="0.8" /><path d="M8 17l-2 4M16 17l2 4M7 20h10" /></Svg>
/** A run over the distance: a speed curve on its axes. */
export const ChartIcon = () => <Svg><path d="M3 3v18h18" /><path d="M6 17c2-7 4-9 6-9s3 4 5 4 2-2 3-4" /></Svg>
export const HomeIcon = () => <Svg><path d="M3 11l9-7 9 7" /><path d="M5 10v10h14V10" /></Svg>
export const SettingsIcon = () => <Svg><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z" /></Svg>
