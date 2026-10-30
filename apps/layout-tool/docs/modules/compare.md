# compare — two states of a project on the map

`src/core/utils/compare/` · needs `@rail-across/core`: `geometry`, `project`, `survey`, `track`

What the core's `merge` module finds between two states, drawn on the map: what is new in the later one green,
what changed orange, what it no longer has grey and dashed — and, for a conflict, the object as each side has it.
The comparison itself (diff, merge) is the core's ([merge](../../../../packages/core/docs/merge.md)); this module
only draws it, with MapLibre.

## Files

| File | What it does |
|---|---|
| `compareLayer.js` | Two states on the map: new green, changed orange, gone grey dashed; a conflict as each side has it |
| `index.js` | The module's public interface |
