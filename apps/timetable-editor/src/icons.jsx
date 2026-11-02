// The Fahrplan-Editor's own symbols, on the suite's 24-unit grid in the
// current colour (the rail draws them white).

const Svg = ({ children }) => (
  <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8"
    strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" focusable="false">
    {children}
  </svg>
)

/** The tool's mark: a graphical timetable — two stations, trains as lines through time —, as the start page's tile. */
export const LogoIcon = () => (
  <svg viewBox="0 0 48 48" width="28" height="28" aria-hidden="true">
    <rect width="48" height="48" rx="12" fill="#303383" />
    <g fill="none" stroke="#fff" strokeWidth="2.6" strokeLinecap="round" strokeLinejoin="round">
      <path d="M12 9 V39 M36 9 V39" strokeOpacity="0.5" strokeWidth="2" />
      <path d="M12 12 L36 22 M12 24 L36 34 M12 37 L36 15" />
    </g>
  </svg>
)
/** A train: its front, two windows, its wheels on the rail. */
export const TrainIcon = () => <Svg><rect x="5" y="3" width="14" height="14" rx="3" /><path d="M5 10h14" /><circle cx="9" cy="14" r="0.8" /><circle cx="15" cy="14" r="0.8" /><path d="M8 17l-2 4M16 17l2 4M7 20h10" /></Svg>
/** The graphical timetable: stations across, trains as lines down through time. */
export const GraphIcon = () => <Svg><path d="M4 3v18M20 3v18" strokeOpacity=".5" /><path d="M4 5l16 7M4 12l16 7M4 20l16-11" /></Svg>
export const HomeIcon = () => <Svg><path d="M3 11l9-7 9 7" /><path d="M5 10v10h14V10" /></Svg>
export const SettingsIcon = () => <Svg><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z" /></Svg>
export const UpIcon = () => <Svg><path d="M6 15l6-6 6 6" /></Svg>
export const DownIcon = () => <Svg><path d="M6 9l6 6 6-6" /></Svg>
export const CopyIcon = () => <Svg><rect x="8" y="8" width="12" height="12" rx="2" /><path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2" /></Svg>
/** A Takt: the same train again and again, at an interval. */
export const TaktIcon = () => <Svg><circle cx="12" cy="13" r="8" /><path d="M12 9v4l3 2M9 2h6" /></Svg>
