// The Signalling-Tool's own symbols, on the suite's 24-unit grid in the
// current colour (the rail draws them white).

const Svg = ({ children }) => (
  <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8"
    strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" focusable="false">
    {children}
  </svg>
)

/** A logical signal: a screen with its two lights. */
export const SignalIcon = () => <Svg><rect x="7" y="2" width="10" height="16" rx="3" /><circle cx="12" cy="7" r="1.6" /><circle cx="12" cy="13" r="1.6" /><path d="M12 18v4" /></Svg>
/** A signal's place: a mast on its foot, a marker at its head. */
export const LocationIcon = () => <Svg><path d="M12 22V9" /><path d="M8 22h8" /><path d="M12 2a4 4 0 0 1 4 4c0 2.5-4 5-4 5s-4-2.5-4-5a4 4 0 0 1 4-4z" /><circle cx="12" cy="6" r="1.3" /></Svg>
/** An axle counting point between two sections. */
export const DetectionIcon = () => <Svg><path d="M2 14h20" /><path d="M12 8v12" /><circle cx="12" cy="6" r="2" /><path d="M5 18h3M16 18h3" /></Svg>
/** A turnout. */
export const SwitchIcon = () => <Svg><path d="M3 16h18" /><path d="M8 16c4 0 7-2 11-8" /><circle cx="8" cy="16" r="1.6" /></Svg>
/** A route from signal to signal. */
export const RouteIcon = () => <Svg><path d="M4 18c5 0 5-12 11-12h5" /><path d="M17 3l3 3-3 3" /><circle cx="4" cy="18" r="2" /></Svg>
/** A balise or track magnet between the rails, sending upwards. */
export const BaliseIcon = () => <Svg><path d="M3 17h18" /><rect x="8" y="11" width="8" height="4" rx="1" /><path d="M10 8l2-2 2 2M10 5l2-2 2 2" /></Svg>
/** A board on its post: a plate with a mark. */
export const BoardIcon = () => <Svg><rect x="6" y="3" width="12" height="9" rx="1" /><path d="M9 7.5h6" /><path d="M12 12v10M8 22h8" /></Svg>
/** A virtual signal: a place without a mast, dashed. */
export const VirtualIcon = () => <Svg><rect x="7" y="3" width="10" height="13" rx="3" strokeDasharray="2.5 2.5" /><path d="M12 16v6" strokeDasharray="2 2" /></Svg>
/** A track section between its two detection points. */
export const SectionIcon = () => <Svg><path d="M2 15h20" /><path d="M5 11v8M19 11v8" /><path d="M5 7h14M7.5 5L5 7l2.5 2M16.5 5L19 7l-2.5 2" /></Svg>
/** A derailer on its rail. */
export const DerailerIcon = () => <Svg><path d="M2 16h20" /><path d="M8 16l4-6h4l-2 6" /></Svg>
/** An overlap: the run on past the signal to its danger point. */
export const OverlapIcon = () => <Svg><path d="M3 16h14" strokeDasharray="0" /><path d="M17 16h4" strokeDasharray="1.5 2" /><path d="M5 16V6" /><rect x="3" y="3" width="4" height="6" rx="1.5" /><path d="M21 12v8" /></Svg>
/** Editing: a pencil. */
export const EditIcon = () => <Svg><path d="M4 20l1-4L16 5l3 3L8 19z" /><path d="M14 7l3 3" /></Svg>
/** Data exchange: a file going out and one coming in. */
export const ExchangeIcon = () => <Svg><path d="M4 8h13M14 4l4 4-4 4" /><path d="M20 16H7M10 12l-4 4 4 4" /></Svg>
/** Map layers: a folded map, as the Layout-Tool draws it. */
export const LayersIcon = () => <Svg><path d="M3 6 L9 3.5 L15 6 L21 3.5 V18 L15 20.5 L9 18 L3 20.5 Z" /><path d="M9 3.5 V18 M15 6 V20.5" /></Svg>
/** The settings: three sliders, as the Layout-Tool draws them. */
export const SettingsIcon = () => <Svg><path d="M3 6 H7 M11 6 H21 M3 12 H14 M18 12 H21 M3 18 H5 M9 18 H21" /><circle cx="9" cy="6" r="2" /><circle cx="16" cy="12" r="2" /><circle cx="7" cy="18" r="2" /></Svg>
/** The check. */
export const CheckIcon = () => <Svg><path d="M4 12l5 5L20 6" /></Svg>
/** A file coming in. */
export const ImportIcon = () => <Svg><path d="M12 3v12M7 10l5 5 5-5" /><path d="M5 21h14" /></Svg>
/** The history: a clock with an arrow running back. */
export const HistoryIcon = () => <Svg><path d="M4 12A8 8 0 1 0 6.3 6.3" /><path d="M2.8 6.6h3.8V2.8" /><path d="M12 7.5V12l3 2" /></Svg>
/** The infrastructure editor of the suite: a house. */
export const HomeIcon = () => <Svg><path d="M2 11.5L12 3l10 8.5" /><path d="M16 6.4V3.5h2.5v5" /><path d="M5 9v10.5A1.5 1.5 0 0 0 6.5 21h11a1.5 1.5 0 0 0 1.5-1.5V9" /></Svg>

/** The tool's mark: a signal — its screen in the current colour with two lights, the mast on its foot. */
export const LogoIcon = () => (
  <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
    <rect x="7" y="1" width="10" height="14" rx="3" fill="currentColor" />
    <circle cx="12" cy="5.2" r="2" fill="#fff" />
    <circle cx="12" cy="10.8" r="2" fill="#fff" fillOpacity="0.55" />
    <path d="M12 15v7M8 22h8" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" />
  </svg>
)
