// The Fahrplan-Editor's own symbols, on the suite's 24-unit grid in the
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
/** A run over the distance: a speed curve on its axes — computing a running time. */
export const ChartIcon = () => <Svg><path d="M3 3v18h18" /><path d="M6 17c2-7 4-9 6-9s3 4 5 4 2-2 3-4" /></Svg>
/** A stopwatch: the running time computed. */
export const TimeIcon = () => <Svg><circle cx="12" cy="14" r="7" /><path d="M12 10v4l2.5 2M10 3h4M12 3v4" /></Svg>
/** The graphical timetable: stations across, trains as lines down through time. */
export const GraphIcon = () => <Svg><path d="M4 3v18M20 3v18" strokeOpacity=".5" /><path d="M4 5l16 7M4 12l16 7M4 20l16-11" /></Svg>
export const LiveIcon = () => <Svg><path d="M3 20c4-2 5-9 9-9s5 7 9 9" /><circle cx="12" cy="11" r="2.4" /><path d="M12 3v3" /></Svg>
export const HomeIcon = () => <Svg><path d="M3 11l9-7 9 7" /><path d="M5 10v10h14V10" /></Svg>
export const SettingsIcon = () => <Svg><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z" /></Svg>
export const UpIcon = () => <Svg><path d="M6 15l6-6 6 6" /></Svg>
export const DownIcon = () => <Svg><path d="M6 9l6 6 6-6" /></Svg>
export const CopyIcon = () => <Svg><rect x="8" y="8" width="12" height="12" rx="2" /><path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2" /></Svg>
/** A Takt: the same train again and again, at an interval. */
export const TaktIcon = () => <Svg><circle cx="12" cy="13" r="8" /><path d="M12 9v4l3 2M9 2h6" /></Svg>
