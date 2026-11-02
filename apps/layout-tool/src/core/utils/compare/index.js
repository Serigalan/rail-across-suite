// compare — two states of a project drawn on the map: what is new, changed or
// gone, and the object as each side of a conflict has it. The comparison itself
// (diff, merge) is the core's.
//
// This file is the module's public face: code outside src/core/utils/compare/
// imports from here and nowhere else (tools/modules.mjs). What is not listed is
// the module's own business. See docs/modules/compare.md.
export {
  clearFeatures, COMPARE_COLORS, comparisonFeatures, comparisonTopology, drawable, findObject,
  objectFeatures, recordFeatures, showFeaturesSoon, trackStylesOf, zoomToFeatures,
} from './compareLayer'
