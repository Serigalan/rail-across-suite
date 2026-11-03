// schematic — the net as a schematic strip along one reference axis: lanes,
// switches as 45° connections, the length along the axis to scale; its lines
// for a lane spacing, and where a station of a track lies on them.
//
// This file is the module's public face: code outside packages/core/src/schematic/
// imports from here and nowhere else (tools/modules.mjs). What is not listed is
// the module's own business. See docs/schematic.md.

export { DEFAULT_CORRIDOR, schematicLayout, schematicNetwork, toPlane } from './layout.js'
export { kmAt, kmMarks, schematicStrip, switchNumber } from './strip.js'
export { schematicPlacer } from './place.js'
