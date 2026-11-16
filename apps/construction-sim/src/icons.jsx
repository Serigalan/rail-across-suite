// The Construction-Sim's own symbols, on the suite's 24-unit grid in the current
// colour (the rail draws them white).

const Svg = ({ children }) => (
  <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8"
    strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" focusable="false">
    {children}
  </svg>
)

/** The tool's mark: a barrier with its lamp across the track — as public/favicon.svg and the start page draw it. */
export const LogoIcon = () => (
  <Svg>
    <path d="M2.5 19h19M7 13v6M17 13v6M7 9V7" />
    <rect x="4" y="9" width="16" height="4" rx="1" />
    <circle cx="7" cy="5.5" r="1.3" fill="currentColor" />
    <path d="M7.5 9.5l-1.2 3M11 9.5l-1.2 3M14.5 9.5l-1.2 3M18 9.5l-1.2 3" strokeOpacity="0.55" />
  </Svg>
)
/** The stages: days on a time line. */
export const StagesIcon = () => <Svg><path d="M2 12h20" /><path d="M6 8v8M12 6v12M18 8v8" strokeOpacity="0.55" /><circle cx="12" cy="12" r="2" fill="currentColor" /></Svg>
/** The construction scenarios: a track with a barrier across it and a way around. */
export const ScenarioIcon = () => <Svg><path d="M2 14h7M15 14h7" /><path d="M9 14c1-5 5-5 6 0" strokeOpacity="0.55" /><path d="M10 11v6M14 11v6" /></Svg>
/** The way back to the suite. */
export const HomeIcon = () => <Svg><path d="M2 11.5L12 3l10 8.5" /><path d="M16 6.4V3.5h2.5v5" /><path d="M5 9v10.5A1.5 1.5 0 0 0 6.5 21h11a1.5 1.5 0 0 0 1.5-1.5V9" /></Svg>
export const SettingsIcon = () => <Svg><circle cx="12" cy="12" r="3" /><path d="M12 2.5v3M12 18.5v3M2.5 12h3M18.5 12h3M5.3 5.3l2.1 2.1M16.6 16.6l2.1 2.1M5.3 18.7l2.1-2.1M16.6 7.4l2.1-2.1" /></Svg>
