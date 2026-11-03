// crossover — the search of the extended switch connection (decision 0018):
// the module of crossover/ (Rust, WebAssembly) beside the page, run in a worker.
//
// This file is the module's public face: code outside src/core/utils/crossover/
// imports from here and nowhere else (tools/modules.mjs). What is not listed is
// the module's own business. See docs/modules/crossover.md.

export { searchCrossover } from './crossoverSearch'
