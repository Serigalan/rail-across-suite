// The vectors of the commands (crates/commands). Their JavaScript is gone
// (decision 0020, step 10) but for the element table's view and the element
// roles, which the page keeps for its renders: the vectors stay as they were
// last recorded from the JavaScript — a change to the table's view goes into
// both, as to any module being ported.
import { frozen } from './record.mjs'

export default function record() {
  return { commands: frozen('commands', 'commands') }
}
