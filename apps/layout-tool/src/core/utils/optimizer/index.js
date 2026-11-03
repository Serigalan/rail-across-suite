// optimizer — the track optimizer (decision 0019): the module of optimizer/
// (Rust, WebAssembly) beside the page, run in a worker of its own — splicing
// and its spacing, reconnecting, optimizing, the alignment fit.
//
// This file is the module's public face: code outside src/core/utils/optimizer/
// imports from here and nowhere else (tools/modules.mjs). What is not listed is
// the module's own business. See docs/modules/optimizer.md.

export { runOptimizer } from './optimizerJob'
export { optimizerRegelwerke } from './optimizerRegelwerke'
