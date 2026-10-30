// A small station signalled with Ks, PZB and ETCS, as the signalling domain holds
// it: on track t1 (250 m) an entry signal A — a Ks multi-section signal with
// Zs 3 and Zs 2 — and an exit signal N1, a Ks main shunting signal; beyond N1
// an overlap over switch w1 onto track t2 (150 m), where a route ends at a
// virtual signal (target on the line). Train detection over t1, flank
// protection for the switch, the PZB magnets of A and N1, an ETCS balise
// group of one at N1, and the station's boundary at A. The tests of the check and the shared vectors start from it.
//
// `operationalPoint` names the station of the core's register it bounds.

export function ksStation({ operationalPoint = 'op1' } = {}) {
  const on = (trackId, station, extra = {}) => ({ anchor: 'station', trackId, station, side: 'right', offset: 3.1, direction: 'up', ...extra })
  return {
    fields: { catalogue: { id: 'db-ril-301', version: '0.3.0' } },
    collections: {
      signals: [
        {
          id: 'sig-a', name: 'A', kind: 'real', function: 'entry', mounting: 'mast',
          logicalSignals: [{ system: 'ks', type: 'multiSection' }],
          at: on('t1', 20), sight: { required: 300, achievable: 420 },
          frames: [
            { kind: 'screen', aspects: [{ aspect: 'Hp_0' }, { aspect: 'Ks_1' }, { aspect: 'Ks_2' }] },
            { kind: 'indicator', aspects: [{ aspect: 'Zs_3', symbol: '6' }] },
            { kind: 'indicator', aspects: [{ aspect: 'Zs_2', symbol: 'H' }] },
            { kind: 'indicator', aspects: [{ aspect: 'Zs_1' }] },
          ],
        },
        {
          id: 'sig-n1', name: 'N1', kind: 'real', function: 'exit', mounting: 'mast',
          logicalSignals: [{ system: 'ks', type: 'mainShunting' }],
          at: on('t1', 230),
          frames: [
            { kind: 'screen', aspects: [{ aspect: 'Hp_0' }, { aspect: 'Ks_1' }, { aspect: 'Sh_1' }] },
            { kind: 'indicator', aspects: [{ aspect: 'Zs_1' }] },
          ],
        },
        {
          id: 'sig-ne1', kind: 'real', function: 'standaloneIndicator', mounting: 'foundation', at: on('t1', 5),
          frames: [{ kind: 'plate', aspects: [{ aspect: 'Ne_1' }] }],
        },
        { id: 'sig-v', kind: 'virtual', virtualFunction: 'trainTargetLine', at: on('t2', 140) },
      ],
      detectionPoints: [
        { id: 'dp-1', name: '1', kind: 'axleCounter', at: on('t1', 15) },
        { id: 'dp-2', name: '2', kind: 'axleCounter', at: on('t1', 240) },
      ],
      trackSections: [{ id: 'ts-1', name: '1', kind: 'axleCounting', boundedBy: ['dp-1', 'dp-2'] }],
      switchElements: [{ id: 'we-1', name: 'W1', switchId: 'w1', operation: 'electricRemote', preferredPosition: 'left' }],
      routes: [
        {
          id: 'rt-a-n1', name: 'A–N1', kind: 'train', trainType: 'ZZ', startSignalId: 'sig-a', targetSignalId: 'sig-n1',
          path: { trackIds: ['t1'] }, speed: 60,
          overlaps: [{ overlapId: 'ov-n1', preferred: true }],
          aspects: [{ signalId: 'sig-a', aspect: 'Ks_1' }, { signalId: 'sig-a', aspect: 'Zs_3', symbol: '6' }],
        },
        {
          id: 'rt-n1-v', name: 'N1–Strecke', kind: 'train', trainType: 'ZZ', startSignalId: 'sig-n1', targetSignalId: 'sig-v',
          path: { trackIds: ['t1', 't2'] }, aspects: [{ signalId: 'sig-n1', aspect: 'Ks_1' }],
        },
      ],
      overlaps: [
        {
          id: 'ov-n1', name: 'N1/200', signalId: 'sig-n1', path: { trackIds: ['t1', 't2'] },
          end: { anchor: 'station', trackId: 't2', station: 100 }, length: 120, gradient: -2.5, speed: 60,
          clearSections: ['ts-1'], switches: [{ switchElementId: 'we-1', locked: true }],
        },
      ],
      flankProtection: [
        { id: 'fla-1', requester: { switchElementId: 'we-1', via: 'left' }, protection: { signalId: 'sig-n1' } },
        { id: 'fla-2', requester: { switchElementId: 'we-1', via: 'right' }, protection: { waived: true } },
      ],
      balises: [
        { id: 'pzb-a-1000', at: on('t1', 20), system: 'pzb', kind: 'magnet', settings: { frequency: '1000' }, signalId: 'sig-a', effect: 'switchedBySignal' },
        { id: 'pzb-n1-500', at: on('t1', 0), system: 'pzb', kind: 'magnet', settings: { frequency: '500', ina: true }, signalId: 'sig-n1', effect: 'switchedBySignal' },
        { id: 'pzb-n1-2000', at: on('t1', 230), system: 'pzb', kind: 'magnet', settings: { frequency: '2000' }, signalId: 'sig-n1', effect: 'switchedBySignal' },
        {
          id: 'pzb-gue', at: on('t1', 200), system: 'pzb', kind: 'speedCheck', signalId: 'sig-n1', effect: 'switchedBySignal',
          settings: { frequency: '2000', testSpeed: 50, measuringLength: 21.4, arrangement: 'startFirst' },
          routes: [{ routeId: 'rt-n1-v', effect: 'inactiveWhenSet' }],
        },
        { id: 'etcs-n1', name: 'N1', at: on('t1', 226), system: 'etcs', kind: 'fixed', settings: { group: 'N1', position: 1 }, signalId: 'sig-n1', effect: 'permanent' },
      ],
      stationBoundaries: [{ id: 'bd-1', operationalPointId: operationalPoint, marks: [{ signalId: 'sig-a' }] }],
    },
  }
}

/** The core the station stands on, as far as the check reads it: two tracks, a switch, a station, two stages. */
export function ksStationCore() {
  const track = (id, length) => ({ id, epsg: 25832, elements: [{ elementType: 0, length }] })
  return {
    fields: { id: 'p', stages: [{ id: 's0' }, { id: 's1' }] },
    collections: {
      tracks: [track('t1', 250), track('t2', 150)],
      switches: [{ id: 'w1', kind: 'turnout', formVersion: 1, ports: {} }],
      operationalPoints: [{ id: 'op1', name: 'Halle-Trotha' }],
    },
  }
}
