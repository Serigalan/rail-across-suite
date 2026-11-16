// Which tool of the suite this page is (decision 0025): the Layout-Tool, which
// lays out the alignment — the core —, or the Civil-Tool, which edits the
// civil layer on it: platforms, runs of tracks, surveys and reference axes,
// the plan header. One build, served under /layout/ and /civil/; each tool
// edits its own and shows the other's. A page run on its own says which with
// `?tool=civil`.

/** 'layout' or 'civil', from the address. */
export function toolOf(loc = globalThis.location) {
  const path = loc?.pathname ?? ''
  const asked = new URLSearchParams(loc?.search ?? '').get('tool')
  return asked === 'civil' || /\/civil\/[^/]*$/.test(path) ? 'civil' : 'layout'
}

/** The tool this page is. */
export const TOOL = toolOf()

/** Whether this page edits the civil layer (the Civil-Tool). */
export const EDITS_CIVIL = TOOL === 'civil'

/** The tool's name, as the page shows it. */
export const TOOL_NAME = EDITS_CIVIL ? 'Civil-Tool' : 'Layout-Tool'
