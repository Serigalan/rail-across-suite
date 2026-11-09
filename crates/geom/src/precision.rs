// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! How finely the alignment is sampled into coordinates (`geometryPrecision.js`).

/// Sagitta [m] of `element.geometry.coordinates`.
pub const SAGITTA_ELEMENT: f64 = 0.05;
/// Sagitta [m] of `renderCoords`.
pub const SAGITTA_TRACK: f64 = 0.2;
/// Sagitta [m] of the polyline an arc or transition is drawn with on the map.
pub const SAGITTA_DISPLAY: f64 = 0.005;
/// Step [m] a switch route's polyline is drawn with.
pub const SWITCH_ROUTE_STEP: f64 = 2.0;
/// Max spacing [m] of the intermediate vertices a straight gets for display.
pub const STRAIGHT_VERTEX_SPACING: f64 = 100.0;
/// Tiling zoom of the map's GeoJSON sources.
pub const GEOJSON_MAXZOOM: f64 = 22.0;
