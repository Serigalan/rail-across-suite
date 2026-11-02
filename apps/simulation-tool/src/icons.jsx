// The Simulation-Tool's own symbols, on the suite's 24-unit grid in the
// current colour (the rail draws them white).

const Svg = ({ children }) => (
  <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8"
    strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" focusable="false">
    {children}
  </svg>
)

/** The tool's mark: a run over the distance — speeding up, running, braking —, as the start page's tile. */
export const LogoIcon = () => (
  <svg viewBox="0 0 48 48" width="28" height="28" aria-hidden="true">
    <rect width="48" height="48" rx="12" fill="#303383" />
    <g fill="none" stroke="#fff" strokeWidth="2.6" strokeLinecap="round" strokeLinejoin="round">
      <path d="M10 11 V37 H39" strokeOpacity="0.5" strokeWidth="2" />
      <path d="M12 36 C14 24 17 18 22 18 H29 C32 18 34 25 36 36" />
      <path d="M10 18 H14" strokeOpacity="0.5" strokeWidth="2" />
    </g>
  </svg>
)
/** A train: its front, two windows, its wheels on the rail. */
export const TrainIcon = () => <Svg><rect x="5" y="3" width="14" height="14" rx="3" /><path d="M5 10h14" /><circle cx="9" cy="14" r="0.8" /><circle cx="15" cy="14" r="0.8" /><path d="M8 17l-2 4M16 17l2 4M7 20h10" /></Svg>
/** A run over the distance: a speed curve on its axes. */
export const ChartIcon = () => <Svg><path d="M3 3v18h18" /><path d="M6 17c2-7 4-9 6-9s3 4 5 4 2-2 3-4" /></Svg>
export const HomeIcon = () => <Svg><path d="M3 11l9-7 9 7" /><path d="M5 10v10h14V10" /></Svg>
export const SettingsIcon = () => <Svg><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z" /></Svg>
