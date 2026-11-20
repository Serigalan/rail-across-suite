/**
 * The transition profiles a run tries unless one is chosen: at the Regelwert
 * the clothoid, the standard form ('bestand': each transition as it lies, a
 * new one a clothoid); at the Ermessensgrenze that and all of them Bloss,
 * the faster one ('auto').
 */
export const defaultTransition = (grenzwert) => (grenzwert === 'discretion' ? 'auto' : 'bestand')
