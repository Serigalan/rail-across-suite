/**
 * Example trains to simulate with, until the suite has its vehicles: the
 * physics a running time needs — mass, length, top speed, the resistance to
 * movement (A + B·v + C·v², v in m/s), the service braking deceleration and
 * the tractive effort over speed. The values are typical of their kind, not
 * those of a type of vehicle.
 */

/** A tractive effort curve: a constant effort up to where the power runs out, then power over speed. */
function effortCurve({ effort, power, maxSpeed }) {
  const out = []
  const corner = power / effort
  for (let v = 0; v <= maxSpeed + 1e-9; v += 1) out.push({ speed: v, max_effort: v <= corner ? effort : power / v })
  if (out[out.length - 1].speed < maxSpeed) out.push({ speed: maxSpeed, max_effort: Math.min(effort, power / maxSpeed) })
  return out
}

const kmh = (v) => v / 3.6

export const VEHICLES = [
  {
    id: 'emu',
    mass: 120_000, length: 70, maxSpeed: kmh(160), inertia: 1.08,
    resistance: { A: 1500, B: 30, C: 5.5 }, deceleration: 0.7,
    effort: { effort: 160_000, power: 2_000_000 },
  },
  {
    id: 'intercity',
    mass: 450_000, length: 230, maxSpeed: kmh(200), inertia: 1.06,
    resistance: { A: 5500, B: 60, C: 9 }, deceleration: 0.6,
    effort: { effort: 300_000, power: 6_400_000 },
  },
  {
    id: 'freight',
    mass: 1_700_000, length: 600, maxSpeed: kmh(100), inertia: 1.05,
    resistance: { A: 22_000, B: 120, C: 14 }, deceleration: 0.3,
    effort: { effort: 300_000, power: 5_600_000 },
  },
]

export const vehicleOf = (id) => VEHICLES.find(v => v.id === id) ?? VEHICLES[0]

/** A vehicle as the simulation takes it. */
export function rollingStockOf(vehicle) {
  return {
    mass: vehicle.mass,
    length: vehicle.length,
    max_speed: vehicle.maxSpeed,
    inertia_coefficient: vehicle.inertia,
    rolling_resistance: vehicle.resistance,
    const_gamma: vehicle.deceleration,
    effort_curve: effortCurve({ ...vehicle.effort, maxSpeed: vehicle.maxSpeed }),
  }
}
