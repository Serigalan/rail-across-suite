import { MAST, screen } from './symbols.js'

/** The type of the symbols' texts: the suite's, and sans-serif wherever it is not there. */
export const SANS = 'var(--ui-font, system-ui), system-ui, sans-serif'

/**
 * The symbols of the schematic view, after the principles of the DB's symbols
 * for signalling plans (Ril 819.9002), not their every measure: a signal
 * folded down along its track in the direction it acts — its foot where it
 * stands, a mast, its screen at the end; a main signal a screen rounded at
 * its end, a multi-section signal one rounded at both, a distant signal one
 * rounded towards the mast; filled, or open (as the plans draw a signal the
 * train works, which the domain does not say yet). A board shows its shape at the
 * end of a mast; axle counters, joints, magnets and balises sit on the track.
 *
 * A symbol is drawn in its own frame, in screen pixels: the foot at the
 * origin, the mast along +x, mirrored by `ax` (-1 where it acts against x),
 * its texts turned back so they read.
 */

/**
 * A text on a symbol, centred on (x, y) and turned the way the signal acts:
 * read with the signal upright — its foot below, its screen above —, its top
 * towards the screen. In a frame mirrored by `ax` it is turned back, so it is
 * never written mirrored.
 */
function Label({ x, y, t, size = 7, ax, color }) {
  return (
    <text transform={`translate(${x},${y}) scale(${ax},1) rotate(${90 * ax})`} textAnchor="middle" dominantBaseline="central"
      fontSize={size} fontWeight={500} fill={color} stroke="none" className="schema-text" style={{ fontFamily: SANS }}>{t}</text>
  )
}

/** A shunting signal's screen: a disc with its bar, as Sh 0 shows it. */
const ShuntingDisc = ({ cx, r, fg, bg }) => (
  <>
    <circle cx={cx} cy="0" r={r} fill={bg} stroke={fg} strokeWidth="1.2" />
    <path d={`M${cx - r * 0.65},0 H${cx + r * 0.65}`} stroke={fg} strokeWidth="1.4" />
  </>
)

/** A signal, a board or a virtual signal, its foot at the origin; `end` is where it ends along x. */
export function SignalSymbol({ symbol, ax, fg, bg }) {
  if (symbol.kind === 'virtual') {
    return <circle cx="4" cy="0" r="4" fill="none" stroke={fg} strokeWidth="1.2" strokeDasharray="2 2" />
  }
  const foot = <path d="M0,-4 V4 M0,0 H10" stroke={fg} strokeWidth="1.2" fill="none" />
  if (symbol.kind === 'board') return <g>{foot}<Board {...symbol} ax={ax} fg={fg} bg={bg} /></g>
  const L = MAST[symbol.family]
  const mast = <path d={`M0,-4 V4 M0,0 H${L}`} stroke={fg} strokeWidth="1.2" fill="none" />
  if (symbol.family === 'shunting') return <g>{mast}<ShuntingDisc cx={L + 4.5} r={4.5} fg={fg} bg={bg} /></g>
  const s = screen(symbol.family, L)
  return (
    <g>
      {mast}
      {symbol.shunt && <ShuntingDisc cx={L - 4.5} r={3} fg={fg} bg={bg} />}
      <path d={s.d} fill={symbol.open ? bg : fg} stroke={fg} strokeWidth="1.2" />
    </g>
  )
}

/** A board at the end of its mast (x 10 on); its text turned the way it acts. */
function Board({ shape, text, hanging, ax, fg, bg }) {
  const box = (w, h = 10) => <rect x="10" y={-h / 2} width={w} height={h} fill={bg} stroke={fg} strokeWidth="1.1" />
  switch (shape) {
    case 'trapezoid': return <path d="M10,-5.5 L19,-3 V3 L10,5.5 Z" fill={bg} stroke={fg} strokeWidth="1.1" />
    case 'cross': return <>{box(10)}<path d="M10,-5 L20,5 M10,5 L20,-5" stroke={fg} strokeWidth="1" /></>
    case 'checker': return <>{box(10)}<path d="M10,-5 H15 V0 H10 Z M15,0 H20 V5 H15 Z" fill={fg} /></>
    case 'halt': return <>{box(14, 9)}<path d="M14,-2.2 H20 M17,-2.2 V2.2 M14,2.2 H20" stroke={fg} strokeWidth="1.1" /></>
    case 'triangle': return (
      <>
        <path d="M10,0 L21,-6 V6 Z" fill={bg} stroke={fg} strokeWidth="1.1" />
        {text && <Label x={17.5} y={0} t={text} size={6} ax={ax} color={fg} />}
      </>
    )
    case 'square': return <>{box(10)}{text && <Label x={15} y={0} t={text} size={7} ax={ax} color={fg} />}</>
    case 'etcsStop': {
      // Ne 14: a white arrow on black, up where it stands alone, down where it hangs.
      const s = hanging ? 1 : -1
      return (
        <>
          <rect x="10" y="-5" width="10" height="10" fill={fg} stroke={fg} strokeWidth="1.1" />
          <path d={`M15,${4 * s} L11.5,${0.5 * s} H13.6 V${-3.5 * s} H16.4 V${0.5 * s} H18.5 Z`} fill={bg} />
        </>
      )
    }
    default: {
      // Any other board: its short name, the box as long across as the text.
      const h = Math.max(10, String(text).length * 4 + 4)
      return <>{box(9, h)}<Label x={14.5} y={0} t={text} size={6} ax={ax} color={fg} /></>
    }
  }
}

/** A detection point on its track: an axle counter's two wheel sensors, a joint across the rails. */
export function DetectionSymbol({ kind, fg, bg }) {
  if (kind === 'axleCounter' || !kind) {
    return <><circle cx="-2.6" cy="0" r="2.4" fill={fg} /><circle cx="2.6" cy="0" r="2.4" fill={fg} /></>
  }
  if (kind === 'insulatedJointOneRail') return <path d="M0,-6 V0" stroke={fg} strokeWidth="1.6" />
  if (kind === 'electricalJoint') return <><path d="M0,-6 V6" stroke={fg} strokeWidth="1.4" /><circle cx="0" cy="-6" r="2" fill={bg} stroke={fg} strokeWidth="1" /></>
  return <path d="M0,-6 V6" stroke={fg} strokeWidth="1.6" />
}

/** What a PZB magnet's frequency reads in the plan. */
const FREQUENCY = { 500: '0,5', 1000: '1', 2000: '2', '1000+2000': '1/2' }

/**
 * A balise or magnet on its track, `sy` the side it lies on (1 below the
 * track on the screen): a PZB magnet a box beside the rail with its
 * frequency, open where the signal switches it; an ETCS data point a box on
 * the track, filled where it is switched.
 */
export function BaliseSymbol({ balise, sy, fg, bg }) {
  const switched = balise.effect === 'switchedBySignal'
  if (balise.system === 'pzb') {
    const f = FREQUENCY[balise.settings?.frequency] ?? ''
    const y0 = sy > 0 ? 0 : -4.5
    return (
      <>
        <rect x="-4" y={y0} width="8" height="4.5" fill={switched ? bg : fg} stroke={fg} strokeWidth="1" />
        {f && <text x="-6" y={sy > 0 ? 8.5 : -2} textAnchor="end" fontSize="7" fill={fg} className="schema-text" style={{ fontFamily: SANS }}>{f}</text>}
      </>
    )
  }
  if (balise.system === 'etcs') return <rect x="-5" y="-2.5" width="10" height="5" fill={switched ? fg : bg} stroke={fg} strokeWidth="1.1" />
  return <rect x="-3" y="-3" width="6" height="6" fill={bg} stroke={fg} strokeWidth="1" />
}

/** A derailer: a half disc on the rail of its side. */
export const DerailerSymbol = ({ sy, fg }) => <path d={`M-4,0 A4,4 0 0 ${sy > 0 ? 0 : 1} 4,0 Z`} fill={fg} />
