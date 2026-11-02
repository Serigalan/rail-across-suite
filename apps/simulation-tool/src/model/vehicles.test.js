import { existsSync, readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import { BUILT_IN } from '@rail-across/vehicles'
import { routeThrough, runWith, simulationRequest } from '@rail-across/running-times'

// OSRD's example vehicles through the module, where it was built (npm run sim)
const WASM = new URL('../../public/sim/rail_across_sim.wasm', import.meta.url)
describe.skipIf(!existsSync(WASM))('OSRD\'s example vehicles', () => {
  const E0 = 500000, N0 = 5600000
  const record = {
    tracks: [{ id: 'T1', epsg: 25832, elements: [{ elementType: 0, startNode: [E0, N0], endNode: [E0 + 8000, N0], bearing: 90, length: 8000, speed: 300 }], heights: [{ station: 0, z: 100 }, { station: 8000, z: 140 }] }],
    switches: [],
  }

  it.each(BUILT_IN.map(e => [e.key, e.vehicle]))('%s runs 8 km up 5 ‰ in every mode', async (key, vehicle) => {
    const { instance } = await WebAssembly.instantiate(readFileSync(WASM))
    const { path, offsets } = routeThrough(record, [{ trackId: 'T1', station: 0, direction: 'up' }, { trackId: 'T1', station: 8000 }])
    for (const mode of Object.keys(vehicle.effort_curves.modes)) {
      const { request } = simulationRequest(record, path, { vehicle, traction: { mode }, stops: [{ offset: offsets[1], duration: 0 }] })
      const result = runWith(instance.exports, request)
      expect(result.error, `${key} ${mode}`).toBeUndefined()
      expect(result.base.positions.at(-1)).toBeCloseTo(8000, 6)
    }
  })
})
