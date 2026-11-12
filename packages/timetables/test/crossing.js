// A test net of the timetables' and the capacity's: a single line with a
// crossing station, signalled both ways, with track sections.

const ks = [{ system: 'ks', type: 'multiSection', function: 'exit', frames: [] }]
// A single line with a crossing station: H-Stadt — line 1 — W-Dorf (track 1 straight, track 2 a little longer) — line 2 — K-Stadt.
const E0 = 500000, N0 = 5600000
const straight = (x0, y0, x1, y1) => ({
  elementType: 0, startNode: [E0 + x0, N0 + y0], endNode: [E0 + x1, N0 + y1], length: Math.hypot(x1 - x0, y1 - y0),
  bearing: (Math.atan2(x1 - x0, y1 - y0) * 180 / Math.PI + 360) % 360, speed: 100,
})
const inStation = (name, number) => ({ trackType: 2, stationName: name, trackNumber: number })
const at = (trackId, station, direction) => ({ anchor: 'station', trackId, station, side: 'right', direction })
const main = (id, trackId, station, direction) => ({ id, name: id, kind: 'real', at: at(trackId, station, direction), logicalSignals: ks })
export const crossing = {
  tracks: [
    { id: 'H1', epsg: 25832, ...inStation('H-Stadt', '1'), elements: [straight(0, 0, 500, 0)] },
    { id: 'L1', epsg: 25832, trackType: 1, lineNumber: '6000', elements: [straight(500, 0, 3000, 0)] },
    { id: 'W1', epsg: 25832, ...inStation('W-Dorf', '1'), elements: [straight(3000, 0, 3600, 0)] },
    { id: 'W2', epsg: 25832, ...inStation('W-Dorf', '2'), elements: [straight(3000, 0, 3300, 60), straight(3300, 60, 3600, 0)] },
    { id: 'L2', epsg: 25832, trackType: 1, lineNumber: '6000', elements: [straight(3600, 0, 6000, 0)] },
    { id: 'K1', epsg: 25832, ...inStation('K-Stadt', '1'), elements: [straight(6000, 0, 6500, 0)] },
  ],
  switches: [
    { switchId: 'Wa', kind: 'turnout', ports: { A: { trackId: 'L1', endpoint: 'END' }, B1: { trackId: 'W2', endpoint: 'BEGIN' }, B2: { trackId: 'W1', endpoint: 'BEGIN' } } },
    { switchId: 'Wb', kind: 'turnout', ports: { A: { trackId: 'L2', endpoint: 'BEGIN' }, B1: { trackId: 'W2', endpoint: 'END' }, B2: { trackId: 'W1', endpoint: 'END' } } },
  ],
  platforms: [
    { id: 'pH', trackId: 'H1', startStation: 100, endStation: 300, stationName: 'H-Stadt', code: '1' },
    { id: 'pK', trackId: 'K1', startStation: 200, endStation: 400, stationName: 'K-Stadt', code: '1' },
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
        { id: 'line1', kind: 'axleCounting', boundedBy: ['a'] },
        { id: 'switchA', kind: 'axleCounting', boundedBy: ['a', 'b', 'c'] },
        { id: 'track1', kind: 'axleCounting', boundedBy: ['b', 'd'] },
        { id: 'track2', kind: 'axleCounting', boundedBy: ['c', 'e'] },
        { id: 'switchB', kind: 'axleCounting', boundedBy: ['d', 'e', 'f'] },
        { id: 'line2', kind: 'axleCounting', boundedBy: ['f'] },
      ],
    },
  },
}
