import { switchBranchSpeed } from '../switch/index.js'
import { portTrack } from '../track/index.js'

/**
 * The turnouts a run of tracks passes on their branch — from the track at
 * the toe (port A) to the one of the branch (B1), or back — and the speed
 * they allow there, the slowest of them: { speed, switchIds }, the speed null
 * where none of them says one (switchBranchSpeed); null where the run passes none
 * on its branch. `switches` are the core's, by `id`, or a record's, by `switchId`.
 */
export function branchSpeedOnRun(switches, trackIds) {
  const switchIds = []
  const speeds = []
  for (let i = 0; i + 1 < (trackIds?.length ?? 0); i++) {
    const pair = [trackIds[i], trackIds[i + 1]]
    for (const sw of switches ?? []) {
      if ((sw.kind ?? 'turnout') !== 'turnout') continue
      const toe = portTrack(sw, 'A'), branch = portTrack(sw, 'B1')
      if (!toe || !branch || toe === branch || !pair.includes(toe) || !pair.includes(branch)) continue
      const id = sw.id ?? sw.switchId
      if (switchIds.includes(id)) continue
      switchIds.push(id)
      const v = switchBranchSpeed(sw)
      if (v > 0) speeds.push(v)
    }
  }
  if (!switchIds.length) return null
  return { speed: speeds.length ? Math.min(...speeds) : null, switchIds }
}
