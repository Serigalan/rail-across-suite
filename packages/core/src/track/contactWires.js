// Where a contact wire runs (electrification 3.0, decision 0029, phase 3): it
// is not kept, it follows from what carries it — on a track, a wire runs from
// its first to its last carrying point there, the suspension points and the
// section insulators that name it. The voltage over a stretch of track is the
// voltage of the wire over it; the operation reads it to know where a vehicle
// runs electrically.

/** The voltages, as OSRD names a traction mode: 15 kV 16.7 Hz first, DB's. */
export const VOLTAGES = ['15000V', '25000V', '3000V', '1500V', '750V', '600V']

/** The empty layer: what a record without one holds of it. */
export const emptyElectrification = () => ({ fields: {}, collections: { contactWires: [], masts: [], suspensionPoints: [], sectionInsulators: [] } })

/**
 * The stretches under contact wire, one per wire and track: [{ contactWireId,
 * voltage, trackId, startStation, endStation }], a wire's stretch on a track
 * from its first to its last carrying point there. `electrification` the
 * layer's domain ({ collections }); a wire nothing carries runs nowhere.
 */
export function wireStretches(electrification) {
  const cols = electrification?.collections ?? {}
  const voltage = new Map((cols.contactWires ?? []).map(w => [w.id, w.voltage]))
  const spans = new Map()
  const carry = (wireId, at) => {
    if (!voltage.has(wireId) || at?.anchor !== 'station' || !Number.isFinite(at.station)) return
    const key = `${wireId}\u0000${at.trackId}`
    const s = spans.get(key) ?? { contactWireId: wireId, voltage: voltage.get(wireId), trackId: at.trackId, startStation: at.station, endStation: at.station }
    s.startStation = Math.min(s.startStation, at.station)
    s.endStation = Math.max(s.endStation, at.station)
    spans.set(key, s)
  }
  for (const p of cols.suspensionPoints ?? []) carry(p.contactWireId, p.at)
  for (const i of cols.sectionInsulators ?? []) {
    carry(i.contactWireBeforeId, i.at)
    carry(i.contactWireAfterId, i.at)
  }
  return [...spans.values()]
}
