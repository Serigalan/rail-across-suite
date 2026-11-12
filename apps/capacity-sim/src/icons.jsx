// The Kapazitäts-Sim's own symbols, on the suite's 24-unit grid in the current
// colour (the rail draws them white).

const Svg = ({ children }) => (
  <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8"
    strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" focusable="false">
    {children}
  </svg>
)

/** The tool's mark: a gauge over the track — as public/favicon.svg and the start page draw it. */
export const LogoIcon = () => (
  <Svg>
    <path d="M4.5 15.5a7.5 7.5 0 0 1 15 0M12 15.5l4-4.25M2.5 19h19" />
    <path d="M6.5 11.1l1 .75M12 8v1.25M17.5 11.1l-1 .75" strokeOpacity="0.55" />
  </Svg>
)
/** A stretch: two stations and the line between them. */
export const StretchIcon = () => <Svg><path d="M2 12h20" /><path d="M4 8v8M20 8v8" /><path d="M8 12l2-3h4l2 3" strokeOpacity="0.55" /></Svg>
/** A station: its tracks side by side. */
export const StationIcon = () => <Svg><path d="M2 8h20M2 16h20" /><path d="M6 8c3 0 3 8 6 8M18 8c-3 0-3 8-6 8" strokeOpacity="0.55" /></Svg>
/** The study: what is looked at. */
export const StudyIcon = () => <Svg><circle cx="10.5" cy="10.5" r="6.5" /><path d="M15.5 15.5L21 21" /></Svg>
/** The way back to the suite. */
export const HomeIcon = () => <Svg><path d="M2 11.5L12 3l10 8.5" /><path d="M16 6.4V3.5h2.5v5" /><path d="M5 9v10.5A1.5 1.5 0 0 0 6.5 21h11a1.5 1.5 0 0 0 1.5-1.5V9" /></Svg>
export const SettingsIcon = () => <Svg><circle cx="12" cy="12" r="3" /><path d="M12 2.5v3M12 18.5v3M2.5 12h3M18.5 12h3M5.3 5.3l2.1 2.1M16.6 16.6l2.1 2.1M5.3 18.7l2.1-2.1M16.6 7.4l2.1-2.1" /></Svg>
