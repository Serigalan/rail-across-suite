import { newBalise } from './domain.js'
import { OFFSET, sideAlong } from './places.js'

/**
 * Equipping signals with PZB magnets: each signal its one magnet at its
 * place, of the frequency its kind asks — a main signal 2000 Hz, a distant
 * signal 1000 Hz, a multi-section signal, which is both, 1000/2000 Hz.
 * Shunting signals, boards and virtual signals get none. A magnet is named
 * after its signal: M, the area's number and the signal's name (21-A: M21A).
 */

/** The frequency a kind of logical signal asks of its magnet. */
const FREQUENCY = {
  main: '2000', mainShunting: '2000', distant: '1000', distantRepeater: '1000', multiSection: '1000+2000', multiSectionShunting: '1000+2000',
}

/** The frequency of the magnet a signal is given, from what its logical signals are; null where it gets none. */
export function magnetFrequency(signal) {
  if (signal.kind !== 'real') return null
  const asked = new Set((signal.logicalSignals ?? []).map(l => FREQUENCY[l.type]).filter(Boolean))
  if (!asked.size) return null
  if (asked.has('1000+2000') || (asked.has('1000') && asked.has('2000'))) return '1000+2000'
  return [...asked][0]
}

/** The name a magnet takes from its signal: M, the area's number and the signal's name, without spaces — 21-A gives M21A; none where the signal has no name. */
export const magnetName = (signal) => (signal?.name ? `M${signal.areaNumber ?? ''}${signal.name}`.replace(/\s+/g, '') : undefined)

/**
 * A balise or magnet given to another signal (or to none): a PZB magnet
 * named after its signal, or not named at all, takes the new one's name.
 */
export function withSignal(balise, signal, before) {
  const { signalId: _old, ...rest } = balise
  const out = signal ? { ...rest, signalId: signal.id } : rest
  if (balise.system !== 'pzb' || (balise.name && balise.name !== magnetName(before))) return out
  const { name: _name, ...bare } = out
  const name = magnetName(signal)
  return name ? { ...bare, name } : bare
}

/** The magnet of a signal: at its station, at the rail on the right of the way it acts, the signal its own, named after it. */
export function signalMagnet(signal, frequency = magnetFrequency(signal)) {
  const at = {
    anchor: 'station', trackId: signal.at.trackId, station: signal.at.station,
    side: sideAlong(signal.at, 'right'), offset: OFFSET.balises, direction: signal.at.direction ?? 'up',
  }
  const magnet = newBalise({ at, system: 'pzb', kind: 'magnet', effect: 'switchedBySignal', signalId: signal.id })
  const name = magnetName(signal)
  return { ...magnet, ...(name ? { name } : {}), settings: { ...magnet.settings, frequency } }
}

/**
 * What equipping the signals would do: `due` the signals on a track that ask
 * for a magnet and have none, each with its frequency; `equipped` how many
 * have one already.
 */
export function magnetsDue(domain) {
  const withMagnet = new Set(domain.collections.balises.filter(b => b.system === 'pzb' && b.signalId).map(b => b.signalId))
  const due = []
  let equipped = 0
  for (const signal of domain.collections.signals) {
    const frequency = magnetFrequency(signal)
    if (!frequency || signal.at?.anchor !== 'station') continue
    if (withMagnet.has(signal.id)) equipped++
    else due.push({ signal, frequency })
  }
  return { due, equipped }
}
