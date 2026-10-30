// The Signalling-Tool's own symbols, on the suite's 24-unit grid in the
// current colour (the rail draws them white).

const Svg = ({ children }) => (
  <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8"
    strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" focusable="false">
    {children}
  </svg>
)

/** A Ks signal: mast and screen with two lights. */
export const SignalIcon = () => <Svg><path d="M12 22V13" /><rect x="8" y="2" width="8" height="11" rx="2" /><circle cx="12" cy="5.5" r="1.4" /><circle cx="12" cy="9.5" r="1.4" /><path d="M8 22h8" /></Svg>
/** An axle counting point between two sections. */
export const DetectionIcon = () => <Svg><path d="M2 14h20" /><path d="M12 8v12" /><circle cx="12" cy="6" r="2" /><path d="M5 18h3M16 18h3" /></Svg>
/** A turnout. */
export const SwitchIcon = () => <Svg><path d="M3 16h18" /><path d="M8 16c4 0 7-2 11-8" /><circle cx="8" cy="16" r="1.6" /></Svg>
/** A route from signal to signal. */
export const RouteIcon = () => <Svg><path d="M4 18c5 0 5-12 11-12h5" /><path d="M17 3l3 3-3 3" /><circle cx="4" cy="18" r="2" /></Svg>
/** Flank protection: a shield. */
export const FlankIcon = () => <Svg><path d="M12 3l7 3v5c0 5-3 8-7 10-4-2-7-5-7-10V6z" /><path d="M9 12l2 2 4-4" /></Svg>
/** A PZB track magnet. */
export const PzbIcon = () => <Svg><path d="M3 17h18" /><rect x="8" y="11" width="8" height="4" rx="1" /><path d="M10 8l2-2 2 2M10 5l2-2 2 2" /></Svg>
/** The check. */
export const CheckIcon = () => <Svg><path d="M4 12l5 5L20 6" /></Svg>
/** A file coming in. */
export const ImportIcon = () => <Svg><path d="M12 3v12M7 10l5 5 5-5" /><path d="M5 21h14" /></Svg>
