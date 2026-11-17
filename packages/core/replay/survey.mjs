// The vectors of the survey module (crates/survey). Its JavaScript is gone
// (decision 0005): the vectors stay as they were last recorded from it.
import { frozen } from './record.mjs'

export default function record() {
  return { survey: frozen('survey', 'survey') }
}
