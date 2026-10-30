// merge — comparing and merging two states of a project (three-way merge,
// id remapping). Drawing the comparison on the map is the app's (compare).
//
// This file is the module's public face: code outside packages/core/src/merge/
// imports from here and nowhere else (tools/modules.mjs). What is not listed is
// the module's own business. See docs/modules/merge.md.

export { diffProject, diffEntries, diffSize, primary } from './diff.js'
export { mergeProject, resolve, validationConflicts } from './merge.js'
