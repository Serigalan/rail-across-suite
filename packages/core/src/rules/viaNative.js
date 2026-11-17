import { callNative, nativeReady } from '../native/index.js'

// The catalogue applied in the core's Rust module (decision 0005):
// what the rules read of an element goes across — its display geometry stays
// here —, and what comes back is the JavaScript's own shape. Only the search
// for the highest speed goes across: it applies the catalogue some fifty
// times per element and answers in two numbers, so Rust is quicker by far;
// checkTrack answers with every value every rule read, and carrying those
// across JSON costs more than evaluating them here (measured on the PEK
// Bestand, 113 elements: 48 ms against 6 ms).

/** What the rules read of an element. */
const READ = ['elementType', 'transitionType', 'radius', 'r1', 'r2', 'length', 'speed', 'cant', 'cantStart', 'cantEnd', 'switchId', 'switchBranch', 'cantException']
const bare = (el) => {
  if (!el || typeof el !== 'object') return el
  const out = {}
  for (const k of READ) if (el[k] !== undefined) out[k] = el[k]
  return out
}

/** The elements and the context as they go across. */
export const rulesInput = (elements, context) => ({
  elements: (elements ?? []).map(bare),
  ...(context ? { context: context.map(c => (c ? bare(c) : null)) } : {}),
})

/** ruleMaxSpeed in Rust, or null where the module is not loaded. */
export function nativeRuleMaxSpeed(elements, i, cap, context) {
  if (!nativeReady()) return null
  return callNative('rules.ruleMaxSpeed', { ...rulesInput(elements, context), i, ...(cap != null ? { cap } : {}) })
}
