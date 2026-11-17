// The vectors of the merge (crates/commands, merge). Its JavaScript is gone
// (decision 0005): the vectors stay as they were last recorded from it.
import { frozen } from './record.mjs'

export default function record() {
  return { commands: { merge: frozen('commands', 'merge') } }
}
