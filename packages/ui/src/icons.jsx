// The suite's generic symbols, drawn on a 24-unit grid in the current colour —
// what every module needs; a module's own symbols are its own.

const Svg = ({ children, size = 18 }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"
    strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" focusable="false">
    {children}
  </svg>
)

export const PlusIcon = (p) => <Svg {...p}><path d="M12 5v14M5 12h14" /></Svg>
export const TrashIcon = (p) => <Svg {...p}><path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3" /></Svg>
export const CloseIcon = (p) => <Svg {...p}><path d="M6 6l12 12M18 6L6 18" /></Svg>
export const CheckIcon = (p) => <Svg {...p}><path d="M5 12l5 5 9-10" /></Svg>
export const BackIcon = (p) => <Svg {...p}><path d="M15 5l-7 7 7 7" /></Svg>
export const UndoIcon = (p) => <Svg {...p}><path d="M9 14L4 9l5-5" /><path d="M4 9h10a6 6 0 0 1 0 12h-3" /></Svg>
export const RedoIcon = (p) => <Svg {...p}><path d="M15 14l5-5-5-5" /><path d="M20 9H10a6 6 0 0 0 0 12h3" /></Svg>
export const UploadIcon = (p) => <Svg {...p}><path d="M12 16V4M7 9l5-5 5 5M5 20h14" /></Svg>
export const DownloadIcon = (p) => <Svg {...p}><path d="M12 4v12M7 11l5 5 5-5M5 20h14" /></Svg>
export const SaveIcon = (p) => <Svg {...p}><path d="M5 4h11l3 3v13H5z" /><path d="M8 4v5h7V4M8 20v-6h8v6" /></Svg>
export const WarningIcon = (p) => <Svg {...p}><path d="M12 3l10 18H2z" /><path d="M12 10v5M12 18v.5" /></Svg>
export const SearchIcon = (p) => <Svg {...p}><circle cx="11" cy="11" r="6" /><path d="M20 20l-4.5-4.5" /></Svg>
export const FitIcon = (p) => <Svg {...p}><path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5" /></Svg>
export const HelpIcon = (p) => <Svg {...p}><circle cx="12" cy="12" r="9" /><path d="M9.5 9.5a2.5 2.5 0 1 1 3.5 2.3c-.7.3-1 .9-1 1.7M12 17v.5" /></Svg>
