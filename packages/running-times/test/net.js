// A small net in UTM 32 with three places, for the tests of the packages
// that run trains: the station A-Dorf (track 1, 500 m, a platform), the line
// 6000 (2500 m) with the halt B-Halt's platform on it, and the station
// C-Stadt behind a turnout — track 1 straight on, track 2 on the branch,
// each with a platform. The line rises 10 m; the design speed is 100 km/h.
const E0 = 500000, N0 = 5600000
const straight = (x0, y0, x1, y1) => ({
  elementType: 0, startNode: [E0 + x0, N0 + y0], endNode: [E0 + x1, N0 + y1], length: Math.hypot(x1 - x0, y1 - y0),
  bearing: (Math.atan2(x1 - x0, y1 - y0) * 180 / Math.PI + 360) % 360, speed: 100,
})
const station = (name, number) => ({ trackType: 2, stationName: name, trackNumber: number })

export const net = {
  tracks: [
    { id: 'A1', epsg: 25832, ...station('A-Dorf', '1'), elements: [straight(0, 0, 500, 0)] },
    { id: 'L', epsg: 25832, trackType: 1, lineNumber: '6000', elements: [straight(500, 0, 3000, 0)], heights: [{ station: 0, z: 100 }, { station: 2500, z: 110 }] },
    { id: 'C1', epsg: 25832, ...station('C-Stadt', '1'), elements: [straight(3000, 0, 3600, 0)] },
    { id: 'C2', epsg: 25832, ...station('C-Stadt', '2'), elements: [straight(3000, 0, 3580, 150)] },
  ],
  switches: [{ switchId: 'W1', kind: 'turnout', ports: { A: { trackId: 'L', endpoint: 'END' }, B1: { trackId: 'C1', endpoint: 'BEGIN' }, B2: { trackId: 'C2', endpoint: 'BEGIN' } } }],
  platforms: [
    { id: 'pA', trackId: 'A1', startStation: 100, endStation: 300, stationName: 'A-Dorf', code: '1' },
    { id: 'pB', trackId: 'L', startStation: 1000, endStation: 1200, stationName: 'B-Halt' },
    { id: 'pC1', trackId: 'C1', startStation: 300, endStation: 500, stationName: 'C-Stadt', code: '1' },
    { id: 'pC2', trackId: 'C2', startStation: 300, endStation: 500, stationName: 'C-Stadt', code: '2' },
  ],
}

/** The simulation module where cargo built it (npm run sim in an app), or null. */
export const WASM = new URL('../../../sim/target/wasm32-unknown-unknown/release/rail_across_sim_wasm.wasm', import.meta.url)
