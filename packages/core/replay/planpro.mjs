// The vectors of the PlanPro reader (crates/planpro). Its JavaScript is gone
// (decision 0020, step 10): the vectors stay as they were last recorded from it.
import { frozen } from './record.mjs'

export default function record() {
  return { planpro: frozen('planpro', 'planpro') }
}
