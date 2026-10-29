# 0004 — Ids on tracks, not on their elements

Status: **accepted**

## Decision

A track has an id; its elements (straights, arcs, transitions) do not. A track's `elements` and its `heights`
stay one value each in diff and merge, as in the Open-Layout-Tool. A height point gets an id only when another
point depends on it, as today.

## Why

- **One editor per track.** A track is edited by one person at a time; two people changing the same track at
  once is so rare that redoing one change is cheaper than everything element ids would cost.
- **Elements depend on each other.** A changed radius moves the nodes of every following element. Merging
  elements one by one would put one side's nodes next to the other's lengths, and the chain checks would turn
  that into a conflict anyway.
- **Nothing refers to an element.** Signals, masts and platforms refer to the track and a station (data model,
  `TrackLocation`).

## What it costs

Two changes to the same track at the same time are a conflict on `elements` (or `heights`): one side is taken,
the other change is made again.

## Alternatives considered

Ids on elements: a finer merge for changes that do not touch the geometry (cant, speed, justifications), paid
for with an id log for splitting and joining elements, larger records and a merge that still has to refuse
chains with gaps.
