// The Fahrzeug-Editor's own symbols, on the suite's 24-unit grid in the
// current colour (the rail draws them white).

const Svg = ({ children }) => (
  <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8"
    strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" focusable="false">
    {children}
  </svg>
)

/**
 * The tool's mark, as the start page draws the Fahrzeug-Editor — a multiple
 * unit from the side — but for the rail's 28 px: thick lines, few of them,
 * the windows filled.
 */
export const LogoIcon = () => (
  <svg viewBox="0 0 32 32" width="28" height="28" aria-hidden="true">
    <g fill="none" stroke="#303383" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M15 11L12.5 7.5L16.5 4.5M13 4.5H20" />
      <path d="M2.5 23Q3.5 19.5 5.6 14.2Q6.4 12.5 8.3 12.5H23.7Q25.6 12.5 26.4 14.2Q28.5 19.5 29.5 23Z" />
      <path d="M9 16h3.5v3H9zM14.3 16h3.5v3h-3.5zM19.6 16h3.5v3h-3.5z" fill="#303383" strokeWidth="1" />
      <path d="M2 27.5H30" strokeOpacity=".45" strokeWidth="1.6" />
    </g>
    <circle cx="8.5" cy="25" r="1.8" fill="#303383" />
    <circle cx="23.5" cy="25" r="1.8" fill="#303383" />
  </svg>
)
/** A train: its front, two windows, its wheels on the rail. */
export const TrainIcon = () => <Svg><rect x="5" y="3" width="14" height="14" rx="3" /><path d="M5 10h14" /><circle cx="9" cy="14" r="0.8" /><circle cx="15" cy="14" r="0.8" /><path d="M8 17l-2 4M16 17l2 4M7 20h10" /></Svg>
export const HomeIcon = () => <Svg><path d="M3 11l9-7 9 7" /><path d="M5 10v10h14V10" /></Svg>
export const SettingsIcon = () => <Svg><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z" /></Svg>
