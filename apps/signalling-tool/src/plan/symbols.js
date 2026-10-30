// How the plan draws what the domain holds — a signal by its kind, a board,
// a clearance marker, a detection point, a PZB magnet — in screen pixels, so
// a symbol stays readable at every zoom.

/** The colour a signal's mast head is drawn in, by what it is. */
export function signalTone(signal) {
  if (signal.kind === 'virtual') return 'virtual'
  const screen = signal.frames?.find(f => f.kind === 'screen')
  if (!screen) {
    const aspects = (signal.frames ?? []).flatMap(f => f.aspects.map(a => a.aspect))
    if (aspects.includes('Ra_12')) return 'marker'
    return 'board'
  }
  if (signal.type === 'distant' || signal.type === 'distantRepeater') return 'distant'
  if (signal.type === 'shunting') return 'shunting'
  return 'main'
}

export const TONE_FILL = {
  main: '#c0392b', distant: '#e2a600', shunting: '#ffffff', board: '#ffffff', marker: '#c0392b', virtual: 'none',
}
