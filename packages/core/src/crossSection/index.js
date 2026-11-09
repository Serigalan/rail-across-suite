// crossSection — the cross section of a track at a station: running plane,
// superstructure and the clearance contours of DB Ril 800.0130.
//
// The module is the core's Rust crate (crates/track, its cross section;
// decision 0021, phase D): every function here is a call of the core's
// module, a track of the project it holds crossing as its name. Its
// constants and the catalogue as the page lists it stay JavaScript.
//
// This file is the module's public face: code outside packages/core/src/crossSection/
// imports from here and nowhere else (tools/modules.mjs). What is not listed is
// the module's own business. See docs/modules/crossSection.md.

import { reuseFrom, viaRust } from '../native/index.js'

export {
  ASSUMED_RAIL_OVER_TERRAIN, DEFAULT_GAUGE_PROFILE, DEFAULT_RAIL, DEFAULT_SLEEPER, GAUGE_PROFILES, gaugeProfile, gaugeProfileLabelKey, LICHTRAUM_SOURCE,
  PLANUM_EDGE, QUERSCHNITT_KATALOG, RAILS, RUNNING_CIRCLE_DISTANCE, SLEEPERS, TRACK_GAUGE,
} from './constants.js'

const op = (name) => viaRust(`crossSection.${name}`, reuseFrom)

export const cantAngle = op('cantAngle')
export const cantPivot = op('cantPivot')
export const crossSection = op('crossSection')
export const elementStartStation = op('elementStartStation')
export const fitSection = op('fitSection')
export const gaugeProfileAreas = op('gaugeProfileAreas')
export const gaugeProfileRing = op('gaugeProfileRing')
export const placeSection = op('placeSection')
export const platformSection = op('platformSection')
export const rotatePoint = op('rotatePoint')
export const rotatePointsAbout = op('rotatePointsAbout')
export const sectionAtStation = op('sectionAtStation')
export const sectionLevels = op('sectionLevels')
export const sectionLinePoints = op('sectionLinePoints')
export const sectionNeighbours = op('sectionNeighbours')
export const sectionOrigin = op('sectionOrigin')
export const superstructureAt = op('superstructureAt')
export const terrainHeightAt = op('terrainHeightAt')
