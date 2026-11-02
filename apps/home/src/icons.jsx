import { LOGO_WORDS } from './logoWords'

// The marks of the suite: its logo; a module's drawing, as wide as its tile on
// the start page; and one mark per module and tool, 48 × 48, beside a name.
// All are thin line drawings in currentColor, their lines as thin at any size.

// The logo: half a rail's profile drawn as one line between two points, the
// rail's axis dashed beside it, and the name. The profile is in the primary
// colour, the name in the text colour (so it reads in the dark as well).
const PROFILE = 'M250 50h-26.6q-16.9 0-16.9 16.9v39.4l24.5 6.5q8.6 2.2 8.6 10.2v75q0 11-10.6 14.5'
  + 'l-56.7 21.2q-3.5 1.3-3.5 4.9v7.7q0 3.7 3.9 3.7H250'

export const Logo = ({ className }) => (
  <svg viewBox="160 40 320 220" className={className} role="img" aria-label="Rail-Across-Suite">
    <path d="M250 50v200" stroke="#888780" strokeDasharray="3 4" opacity=".6" fill="none" vectorEffect="non-scaling-stroke" />
    <path d={PROFILE} fill="none" stroke="var(--color-primary)" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"
      vectorEffect="non-scaling-stroke" />
    <circle cx="250" cy="50" r="5" fill="var(--color-primary)" />
    <circle cx="250" cy="250" r="5" fill="var(--color-primary)" />
    <path d={LOGO_WORDS} fill="currentColor" />
  </svg>
)

// A drawing: its lines in currentColor, as thin at any size (home.css).
const Drawing = ({ width, height, className, children }) => (
  <svg viewBox={`0 0 ${width} ${height}`} className={className} aria-hidden="true">
    <g fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">{children}</g>
  </svg>
)
const Dot = ({ x, y, r = 2.2 }) => <circle cx={x} cy={y} r={r} fill="currentColor" stroke="none" />
// A line drawn faint: the ground the drawing stands on — stations, sleepers, stripes.
const Faint = (props) => <path strokeOpacity="0.45" {...props} />
// A train in the graphical timetable, in one of its tones (home.css).
const Train = ({ tone, d }) => <path d={d} style={{ stroke: `var(--ra-tone-${tone}, currentColor)` }} />
// A switch blade: the small filled wedge where a crossover leaves its track —
// at the joint (x, y), the crossover heading (dx, dy).
const Blade = ({ x, y, dx, dy, s = 2.6 }) => <path d={`M${x} ${y}h${dx * s}l0 ${dy * s}z`} fill="currentColor" stroke="none" />
// Sleepers under a track at y, every 8 from x0 to x1.
const sleepers = (y, x0, x1) => Array.from({ length: Math.floor((x1 - x0) / 8) + 1 }, (_, i) => `M${x0 + i * 8} ${y}v3`).join('')

const MARKS = {
  // Three tracks, crossovers between them, the top one branching off.
  infrastructure: (
    <>
      <path d="M4 17H22L33 6M4 27H44M4 37H44M10 17L20 27M38 27L28 37" />
      <Blade x={10} y={17} dx={1} dy={1} /><Blade x={20} y={27} dx={-1} dy={-1} />
      <Blade x={38} y={27} dx={-1} dy={1} /><Blade x={28} y={37} dx={1} dy={-1} />
    </>
  ),
  // A multiple unit from the side: its pantograph, windows, door and bogies.
  vehicles: (
    <>
      <path d="M23 17L19.5 13L25 9M20.5 9H29" />
      <rect x="12" y="16" width="24" height="2.4" rx="1.2" />
      <path d="M4 33Q5 30 8.5 22Q9.5 20 12 20H36Q38.5 20 39.5 22Q43 30 44 33V35Q44 36 43 36H5Q4 36 4 35ZM4 33H44" />
      <path d="M7 30L9.6 24Q10 23 11 23V30ZM41 30L38.4 24Q38 23 37 23V30Z" />
      <rect x="13" y="23" width="5" height="5" rx="1.2" /><rect x="19.5" y="23" width="5" height="5" rx="1.2" />
      <rect x="26.5" y="22.5" width="4.5" height="13.5" rx=".6" /><rect x="32.5" y="23" width="3" height="5" rx="1" />
      <path d="M7 36a2.2 2.2 0 0 0 4.4 0M12.6 36a2.2 2.2 0 0 0 4.4 0M31 36a2.2 2.2 0 0 0 4.4 0M36.6 36a2.2 2.2 0 0 0 4.4 0" />
      <Faint d="M3 40.5H45" />
    </>
  ),
  // The graphical timetable: three stations across, trains through time, stopping at the middle one.
  timetable: (
    <>
      <Faint d="M8 5V43M24 5V43M40 5V43" />
      <Train tone={1} d="M8 7L24 17V21L40 31" />
      <Train tone={3} d="M8 19L24 29V33L40 43" />
      <Train tone={2} d="M40 9L24 21V25L8 37" />
    </>
  ),
  // Capacity: a gauge over the track — how far the line is filled.
  capacitySim: (
    <>
      <path d="M9 31a15 15 0 0 1 30 0M24 31L32 22.5M5 38H43" />
      <Faint d={`M12.9 22.2l2 1.5M24 16v2.5M35.1 22.2l-2 1.5${sleepers(38, 9, 41)}`} />
      <Dot x={24} y={31} />
    </>
  ),
  // A construction site: a barrier with its lamp across the track.
  constructionSim: (
    <>
      <circle cx="14" cy="9" r="2.4" />
      <path d="M14 11.4V14M14 22V35M34 22V35M5 35H43" />
      <rect x="8" y="14" width="32" height="8" rx="1.5" />
      <Faint d={`M15 14l-4 8M22 14l-4 8M29 14l-4 8M36 14l-4 8${sleepers(35, 9, 41)}`} />
    </>
  ),
  // The Layout-Tool: an alignment of a straight, a transition and an arc leaving the straight, its elements' ends marked.
  layout: (
    <>
      <Faint d="M17 36H44" strokeDasharray="2 2.5" />
      <path d="M4 36H17C25 36 28 31 31 25C34 18 37 13 44 11" />
      <Dot x={17} y={36} /><Dot x={31} y={25} />
    </>
  ),
  // The Signalling-Tool: a main signal on its mast, the mast's plate on it.
  signalling: (
    <>
      <rect x="16" y="5" width="16" height="22" rx="4" />
      <Dot x={24} y={11.5} r={3} />
      <circle cx="24" cy="20" r="2.6" />
      <path d="M24 27V43M17 43H31" />
      <rect x="21" y="31" width="6" height="7" rx=".8" />
      <path d="M21 33.3h6v2.4h-6z" fill="currentColor" stroke="none" />
    </>
  ),
  // The Catenary-Tool: a mast, its cantilever, the catenary wire with its droppers and the contact wire.
  catenary: (
    <>
      <path d="M10 43V5M6 43H14M10 10H19M10 19L19 12M19 11C26 18 36 18 45 13M15 25H45" />
      <Faint d="M25 15.9V25M32 17V25M39 16V25" />
    </>
  ),
}

// The modules' drawings, 120 × 48, as wide as their tiles: what each is about,
// drawn as a plan or from the side.
const ART = {
  // Three tracks, the top one branching off, crossovers between them both ways — every line running out of the drawing.
  infrastructure: (
    <>
      <path d="M0 16H84L124 -24M0 28H120M0 40H120M18 16L30 28M54 28L42 40M74 16L62 28M86 28L98 40" />
      <Blade x={18} y={16} dx={1} dy={1} s={3} /><Blade x={30} y={28} dx={-1} dy={-1} s={3} />
      <Blade x={54} y={28} dx={-1} dy={1} s={3} /><Blade x={42} y={40} dx={1} dy={-1} s={3} />
      <Blade x={74} y={16} dx={-1} dy={1} s={3} /><Blade x={62} y={28} dx={1} dy={-1} s={3} />
      <Blade x={86} y={28} dx={1} dy={1} s={3} /><Blade x={98} y={40} dx={-1} dy={-1} s={3} />
    </>
  ),
  // A multiple unit from the side: pantograph and roof, windows, doors, its bogies on the rail.
  vehicles: (
    <>
      <g transform="translate(20.3 6.8) scale(0.21) translate(-68 -62)">
        <path d="M268 62h86m-21 0v9" /><circle cx="333" cy="79" r="8" /><path d="m328 86-7 17" />
        <rect x="125" y="103" width="264" height="12" rx="5" />
        <path d="M68 205q6-15 30-77 6-12 24-12h270q18 0 24 12 24 62 30 77v15q0 6-6 6H74q-6 0-6-6Zm0 0h378" />
        <path d="m82 190 16-42q3-6 10-6v48Z" />
        <rect x="120" y="140" width="62" height="50" rx="10" /><rect x="192" y="140" width="62" height="50" rx="10" />
        <rect x="266" y="136" width="54" height="90" rx="2" /><rect x="278" y="150" width="30" height="30" rx="7" />
        <rect x="332" y="140" width="62" height="50" rx="10" />
        <path d="m432 190-16-42q-3-6-10-6v48ZM97 226a13 13 0 0 0 26 0m44 0a13 13 0 0 0 26 0m128 0a13 13 0 0 0 26 0m44 0a13 13 0 0 0 26 0" />
      </g>
      <Faint d="M0 44H120" />
    </>
  ),
  // The graphical timetable: three stations, trains in their interval both ways, stopping at the middle one,
  // each kind in its own tone.
  timetable: (
    <>
      <Faint d="M0 6H120M0 24H120M0 42H120" />
      <Train tone={1} d="M8 6L30 24H36L58 42M88 6L110 24H116L120 27.3" />
      <Train tone={3} d="M48 6L70 24H76L98 42M0 22.4L2 24H10L32 42" />
      <Train tone={2} d="M24 42L46 24H50L72 6M64 42L86 24H90L112 6" />
    </>
  ),
  // Capacity: trains following each other on the line, the headway between them measured.
  capacitySim: (
    <>
      <path d="M0 36H120" />
      <Faint d={sleepers(36, 4, 116)} />
      <path d="M6 33V22Q6 19 9 19H26Q29 19 30.5 22L32 26V33ZM46 33V22Q46 19 49 19H66Q69 19 70.5 22L72 26V33ZM86 33V22Q86 19 89 19H106Q109 19 110.5 22L112 26V33Z" />
      <Faint d="M10 23h5v4h-5zM18 23h5v4h-5zM50 23h5v4h-5zM58 23h5v4h-5zM90 23h5v4h-5zM98 23h5v4h-5zM32 8V16M46 8V16M72 8V16M86 8V16" />
      <path d="M33 11H45M73 11H85M36 8.5L33 11L36 13.5M42 8.5L45 11L42 13.5M76 8.5L73 11L76 13.5M82 8.5L85 11L82 13.5" />
    </>
  ),
  // A construction site: a speed restriction's board before it, a barrier with its lamp across the track.
  constructionSim: (
    <>
      <path d="M0 38H120" />
      <Faint d={sleepers(38, 4, 116)} />
      <path d="M16 38V20M9 6H23L16 18Z" />
      <text x="16" y="13.4" fontSize="6.5" fontWeight="700" textAnchor="middle" fill="currentColor" stroke="none">6</text>
      <circle cx="58" cy="9" r="2.4" />
      <path d="M58 11.4V14M57 23V38M87 23V38" />
      <rect x="50" y="14" width="44" height="9" rx="1.5" />
      <Faint d="M57 14l-4.5 9M65 14l-4.5 9M73 14l-4.5 9M81 14l-4.5 9M89 14l-4.5 9" />
    </>
  ),
}

/** The mark of a module or tool, by its id. */
export const ModuleIcon = ({ id }) => (MARKS[id] ? <Drawing width={48} height={48} className="ra-icon">{MARKS[id]}</Drawing> : null)

/** A module's drawing, as wide as its tile on the start page, by its id. */
export const ModuleArt = ({ id }) => (ART[id] ? <Drawing width={120} height={48} className="ra-art">{ART[id]}</Drawing> : null)
