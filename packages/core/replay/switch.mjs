// The switch crate's vectors (crates/switch): the catalogue and routes,
// crossings and the symbol, sleepers, the turnout plane, detail, deletion,
// placement and the switch connection's solvers. Frozen: the module's
// JavaScript is gone (decision 0021, phase D) — they were recorded from it and
// hold the crate to what it answered.
import { frozen } from './record.mjs'

export default function record() {
  return { switch: frozen('switch', 'switch') }
}
