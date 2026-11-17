// The vectors of the PlanPro reader (crates/planpro), as last recorded from its
// JavaScript (decision 0005). The reader is the check of the PlanPro
// export now (decision 0010) and no part of the module: the crate replays them
// itself (crates/planpro/tests/replay.rs).
import { frozen } from './record.mjs'

export default function record() {
  return { planpro: frozen('planpro', 'planpro') }
}
