// A test net of the timetables' and the capacity's: a single line with a
// crossing station, signalled both ways, with track sections.

const ks = [{ system: 'ks', type: 'multiSection', function: 'exit', frames: [] }]
// A single line with a crossing station: H-Stadt — line 1 — W-Dorf (track 1 straight, track 2 a little longer) — line 2 — K-Stadt.
const E0 = 500000, N0 = 5600000
const straight = (x0, y0, x1, y1) => ({
  elementType: 'straight', startNode: [E0 + x0, N0 + y0], endNode: [E0 + x1, N0 + y1], length: Math.hypot(x1 - x0, y1 - y0),
  bearing: (Math.atan2(x1 - x0, y1 - y0) * 180 / Math.PI + 360) % 360, speed: 100,
})
// The stations are points of the register, each track and platform naming its own (decision 0006).
const inStation = (point, number) => ({ trackType: 'station', operationalPointId: point, trackNumber: number })
const at = (trackId, station, direction) => ({ anchor: 'station', trackId, station, side: 'right', direction })
const main = (id, trackId, station, direction) => ({ id, name: id, kind: 'real', at: at(trackId, station, direction), logicalSignals: ks })
export const crossing = {
  tracks: [
    { id: 'H1', epsg: 25832, ...inStation('op-h', '1'), elements: [straight(0, 0, 500, 0)] },
    { id: 'L1', epsg: 25832, trackType: 'line', lineNumber: '6000', elements: [straight(500, 0, 3000, 0)] },
    { id: 'W1', epsg: 25832, ...inStation('op-w', '1'), elements: [straight(3000, 0, 3600, 0)] },
    { id: 'W2', epsg: 25832, ...inStation('op-w', '2'), elements: [straight(3000, 0, 3300, 60), straight(3300, 60, 3600, 0)] },
    { id: 'L2', epsg: 25832, trackType: 'line', lineNumber: '6000', elements: [straight(3600, 0, 6000, 0)] },
    { id: 'K1', epsg: 25832, ...inStation('op-k', '1'), elements: [straight(6000, 0, 6500, 0)] },
  ],
  switches: [
    { switchId: 'Wa', kind: 'switch', ports: { A: { trackId: 'L1', endpoint: 'END' }, B1: { trackId: 'W2', endpoint: 'BEGIN' }, B2: { trackId: 'W1', endpoint: 'BEGIN' } } },
    { switchId: 'Wb', kind: 'switch', ports: { A: { trackId: 'L2', endpoint: 'BEGIN' }, B1: { trackId: 'W2', endpoint: 'END' }, B2: { trackId: 'W1', endpoint: 'END' } } },
  ],
  platforms: [
    { id: 'pH', trackId: 'H1', startStation: 100, endStation: 300, operationalPointId: 'op-h' },
    { id: 'pK', trackId: 'K1', startStation: 200, endStation: 400, operationalPointId: 'op-k' },
  ],
  operationalPoints: [{ id: 'op-h', name: 'H-Stadt' }, { id: 'op-w', name: 'W-Dorf' }, { id: 'op-k', name: 'K-Stadt' }],
  // where trains stop: at the two platforms
  stoppingPlaces: [
    { id: 'hH', name: '1', operationalPointId: 'op-h', at: { anchor: 'station', trackId: 'H1', startStation: 100, endStation: 300 } },
    { id: 'hK', name: '1', operationalPointId: 'op-k', at: { anchor: 'station', trackId: 'K1', startStation: 200, endStation: 400 } },
  ],
  signalling: {
    fields: {},
    collections: {
      signals: [
        main('HN', 'H1', 450, 'up'), main('WF', 'L1', 2300, 'up'), main('WN1', 'W1', 550, 'up'), main('WN2', 'W2', 560, 'up'), main('KF', 'L2', 2200, 'up'),
        main('KP', 'K1', 50, 'down'), main('WA', 'L2', 200, 'down'), main('WP1', 'W1', 50, 'down'), main('WP2', 'W2', 50, 'down'), main('HA', 'L1', 300, 'down'),
      ],
      routes: [
        { id: 'WF-WN1', kind: 'train', startSignalId: 'WF', targetSignalId: 'WN1', path: { trackIds: ['L1', 'W1'] } },
        { id: 'WF-WN2', kind: 'train', startSignalId: 'WF', targetSignalId: 'WN2', path: { trackIds: ['L1', 'W2'] } },
        { id: 'WA-WP1', kind: 'train', startSignalId: 'WA', targetSignalId: 'WP1', path: { trackIds: ['L2', 'W1'] } },
        { id: 'WA-WP2', kind: 'train', startSignalId: 'WA', targetSignalId: 'WP2', path: { trackIds: ['L2', 'W2'] } },
      ],
      overlaps: [],
      // Axle counters at the line's ends, by the switches and the signals: the switch areas, the station tracks, the lines each a section of their own.
      detectionPoints: [
        ['a', 'L1', 2350], ['b', 'W1', 20], ['c', 'W2', 20], ['d', 'W1', 580], ['e', 'W2', 590], ['f', 'L2', 150],
      ].map(([id, trackId, station]) => ({ id, kind: 'axleCounter', at: at(trackId, station) })),
      trackSections: [
        { id: 'line1', kind: 'axleCounting', detectionPointIds: ['a'] },
        { id: 'switchA', kind: 'axleCounting', detectionPointIds: ['a', 'b', 'c'] },
        { id: 'track1', kind: 'axleCounting', detectionPointIds: ['b', 'd'] },
        { id: 'track2', kind: 'axleCounting', detectionPointIds: ['c', 'e'] },
        { id: 'switchB', kind: 'axleCounting', detectionPointIds: ['d', 'e', 'f'] },
        { id: 'line2', kind: 'axleCounting', detectionPointIds: ['f'] },
      ],
    },
  },
}
