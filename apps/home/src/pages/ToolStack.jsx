import { ModuleIcon } from '../icons'

/**
 * The marks of some tools as a small stack, each on its own round ground and
 * overlapping the one before — what is behind a "more" or a "back", at a
 * glance. A tool still to come stands greyed, the `chosen` one ringed.
 */
export default function ToolStack({ tools, chosen, vertical = false }) {
  return (
    <span className={`ra-stack${vertical ? ' ra-stack-vertical' : ''}`} aria-hidden="true">
      {tools.map(tool => (
        <span key={tool.id} className={`ra-stack-icon${tool.available ? '' : ' ra-stack-planned'}${tool.id === chosen ? ' ra-stack-chosen' : ''}`}><ModuleIcon id={tool.id} /></span>
      ))}
    </span>
  )
}
