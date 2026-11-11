// The Catenary-Tool's own symbols, on the suite's 24-unit grid in the current
// colour (the rail draws them white).

const Svg = ({ children }) => (
  <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8"
    strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" focusable="false">
    {children}
  </svg>
)

/** A mast with its cantilever and the contact wire over the track: the tool's mark. */
export const LogoIcon = () => <Svg><path d="M5 22V3" /><path d="M5 6h13" /><path d="M5 9l9-3" /><path d="M2 12h20" /><path d="M14 6v6" /><path d="M2 20h20" /></Svg>
/** A range under contact wire: the wire over a stretch of track, a bolt. */
export const RangeIcon = () => <Svg><path d="M2 6h20" /><path d="M2 19h20" /><path d="M7 6v3M17 6v3" /><path d="M13 9l-3 5h4l-3 5" /></Svg>
export const CheckIcon = () => <Svg><path d="M4 12l5 5L20 6" /></Svg>
export const SettingsIcon = () => <Svg><path d="M3 6 H7 M11 6 H21 M3 12 H14 M18 12 H21 M3 18 H5 M9 18 H21" /><circle cx="9" cy="6" r="2" /><circle cx="16" cy="12" r="2" /><circle cx="7" cy="18" r="2" /></Svg>
export const HomeIcon = () => <Svg><path d="M2 11.5L12 3l10 8.5" /><path d="M16 6.4V3.5h2.5v5" /><path d="M5 9v10.5A1.5 1.5 0 0 0 6.5 21h11a1.5 1.5 0 0 0 1.5-1.5V9" /></Svg>
