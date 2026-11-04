/**
 * The tracks a form of the edit panel offers in its list: not those that are
 * only a switch's own geometry — they go with their switch, not on their own,
 * and the map leaves them out of the pick the same way (noSwitchBranch).
 */
export const pickableTracks = (tracks) =>
  (tracks ?? []).filter(tr => !(tr.elements?.length && tr.elements.every(el => el.switchBranch === true || el.switchBranch === 'true')))

/** A track as such a form names it. */
export const trackTitle = (track) => track.name || track.id.slice(0, 8)
