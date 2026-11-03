// splice — "Zwei Gleise zusammenfügen": what a splice can change where the
// rules reject a curvature jump. The construction itself is the optimizer
// module's (utils/optimizer, decision 0019).
//
// This file is the module's public face: code outside src/core/utils/splice/
// imports from here and nowhere else (tools/modules.mjs). What is not listed is
// the module's own business. See docs/modules/splice.md.

export { hasJumpError, spliceRemedies } from './spliceRemedies'
