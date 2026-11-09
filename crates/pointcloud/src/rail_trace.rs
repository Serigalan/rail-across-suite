// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Finding a track's rail heads along a guide in the project's clouds
//! (`railTrace.js`, phase 12): a track of the project or a line drawn by
//! hand, walked in steps, each axis point exported.

use rail_across_js::{js_hypot, js_round, js_to_fixed};
use rail_across_track::cross_section::section_origin;
use serde_json::{Value, json};

use crate::rail_detect::{Detection, detect_track};
use crate::section::{Cloud, cloud_section_points};
use crate::{Error, Result};

const DETECT_THICKNESS: f64 = 0.5;
pub const TRACE_STEP: f64 = 0.5;
const GUIDE_WINDOW: f64 = 1.5;
const FOLLOW_WINDOW: f64 = 0.1;
const FOLLOW_REACH: f64 = 4.0;
const FOLLOW_MIN: usize = 3;
const SEARCH_TURNS: [f64; 5] = [0.0, 3.0, -3.0, 6.0, -6.0];
const DEG: f64 = std::f64::consts::PI / 180.0;

/// The heads in the clouds at one station (`detectInClouds`).
pub fn detect_in_clouds(clouds: &[Cloud], origin: (f64, f64), bearing: f64, crs: i64, around: f64, window: f64, rail: Option<&str>) -> Result<Detection> {
    let half_width = around.abs() + window + 1.2;
    let (mut y, mut z) = (Vec::new(), Vec::new());
    for cloud in clouds {
        let p = cloud_section_points(cloud, origin, bearing, crs, half_width, DETECT_THICKNESS)?;
        y.extend(p.y);
        z.extend(p.z);
    }
    Ok(detect_track(&y, &z, around, window, rail))
}

/// A guide: plane, length, window and the point at a station.
pub enum Guide {
    Track(Value),
    Line { pts: Vec<[f64; 2]>, cum: Vec<f64>, epsg: Value },
}

impl Guide {
    /// A guide along a track of the project (`trackGuide`).
    pub fn track(track: &Value) -> Guide {
        Guide::Track(track.clone())
    }

    /// A guide along a polyline drawn by hand (`lineGuide`).
    pub fn line(vertices: &[[f64; 2]], epsg: Value) -> Guide {
        let pts: Vec<[f64; 2]> = vertices
            .iter()
            .enumerate()
            .filter(|(i, v)| *i == 0 || js_hypot(&[v[0] - vertices[i - 1][0], v[1] - vertices[i - 1][1]]) > 1e-6)
            .map(|(_, v)| *v)
            .collect();
        let mut cum = vec![0.0];
        for i in 1..pts.len() {
            cum.push(cum[i - 1] + js_hypot(&[pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]]));
        }
        Guide::Line { pts, cum, epsg }
    }

    pub fn epsg(&self) -> Value {
        match self {
            Guide::Track(t) => t.get("epsg").cloned().unwrap_or(Value::Null),
            Guide::Line { epsg, .. } => epsg.clone(),
        }
    }

    pub fn length(&self) -> f64 {
        match self {
            Guide::Track(t) => rail_across_track::heights::track_length(t),
            Guide::Line { cum, .. } => *cum.last().unwrap_or(&f64::NAN),
        }
    }

    pub fn window(&self) -> f64 {
        GUIDE_WINDOW
    }

    /// The point (easting, northing, bearing) at station `s`.
    pub fn at(&self, s: f64) -> Option<(f64, f64, f64)> {
        match self {
            Guide::Track(t) => section_origin(t, s).map(|(u, b)| (u.e, u.n, b)),
            Guide::Line { pts, cum, .. } => {
                if pts.len() < 2 {
                    return None;
                }
                let mut i = 1;
                while i < pts.len() - 1 && cum[i] < s {
                    i += 1;
                }
                let (a, b) = (pts[i - 1], pts[i]);
                let t = ((s - cum[i - 1]) / (cum[i] - cum[i - 1])).clamp(0.0, 1.0);
                Some((
                    a[0] + (b[0] - a[0]) * t,
                    a[1] + (b[1] - a[1]) * t,
                    ((b[0] - a[0]).atan2(b[1] - a[1]) * 180.0 / std::f64::consts::PI + 360.0) % 360.0,
                ))
            }
        }
    }
}

/// A traced point.
#[derive(Clone, Debug)]
pub struct Point {
    pub station: f64,
    pub easting: f64,
    pub northing: f64,
    pub offset: f64,
    pub turn: f64,
    pub z_left: f64,
    pub z_right: f64,
    pub cant: f64,
    pub gauge: f64,
    pub quality: String,
}

impl Point {
    pub fn to_json(&self) -> Value {
        let n = rail_across_js::number;
        json!({
            "station": n(self.station), "easting": n(self.easting), "northing": n(self.northing), "offset": n(self.offset),
            "turn": n(self.turn), "zLeft": n(self.z_left), "zRight": n(self.z_right), "cant": n(self.cant), "gauge": n(self.gauge),
            "quality": self.quality,
        })
    }

    pub fn from_json(v: &Value) -> Point {
        let f = |k: &str| v.get(k).and_then(Value::as_f64).unwrap_or(f64::NAN);
        Point {
            station: f("station"),
            easting: f("easting"),
            northing: f("northing"),
            offset: f("offset"),
            turn: v.get("turn").and_then(Value::as_f64).unwrap_or(0.0),
            z_left: v
                .get("zLeft")
                .or_else(|| v.get("left").and_then(|l| l.get("z")))
                .and_then(Value::as_f64)
                .unwrap_or(f64::NAN),
            z_right: v
                .get("zRight")
                .or_else(|| v.get("right").and_then(|l| l.get("z")))
                .and_then(Value::as_f64)
                .unwrap_or(f64::NAN),
            cant: f("cant"),
            gauge: f("gauge"),
            quality: v.get("quality").and_then(Value::as_str).unwrap_or("").to_string(),
        }
    }
}

/// Where the next axis point should be (`foresee`): (offset, slope).
pub fn foresee(points: &[Point], station: f64) -> Option<(f64, f64)> {
    let mut recent: Vec<&Point> = Vec::new();
    for p in points.iter().rev() {
        if station - p.station > FOLLOW_REACH {
            break;
        }
        recent.push(p);
    }
    if recent.is_empty() {
        return None;
    }
    if recent.len() < FOLLOW_MIN {
        let last = recent[0];
        let slope = (last.turn * DEG).tan();
        return Some((last.offset + slope * (station - last.station), slope));
    }
    let n = recent.len() as f64;
    let ms = recent.iter().fold(0.0, |a, p| a + p.station) / n;
    let mo = recent.iter().fold(0.0, |a, p| a + p.offset) / n;
    let sxx = recent.iter().fold(0.0, |a, p| a + (p.station - ms).powi(2));
    let slope = if sxx > 0.0 {
        recent.iter().fold(0.0, |a, p| a + (p.station - ms) * (p.offset - mo)) / sxx
    } else {
        0.0
    };
    Some((mo + slope * (station - ms), slope))
}

/// Stations without a track grouped into stretches (`gapsOf`).
pub fn gaps_of(stations: &[f64], step: f64) -> Vec<(f64, f64)> {
    let mut out: Vec<(f64, f64)> = Vec::new();
    for &s in stations {
        match out.last_mut() {
            Some(last) if s - last.1 <= step * 1.5 => last.1 = s,
            _ => out.push((s, s)),
        }
    }
    out
}

/// What a detection at one station is asked: origin, bearing, plane, around, window.
pub type Detect<'a> = dyn FnMut((f64, f64), f64, &Value, f64, f64) -> Result<Detection> + 'a;

/// Walk `guide` and find the track's heads at every step (`traceTrack`).
pub fn trace_track(guide: &Guide, step: Option<f64>, detect: &mut Detect, on_progress: &mut dyn FnMut(f64), aborted: &dyn Fn() -> bool) -> Result<Value> {
    let step = step.unwrap_or(TRACE_STEP);
    let mut points: Vec<Point> = Vec::new();
    let mut missed = Vec::new();
    let length = guide.length();
    let steps = ((length / step + 1e-9).floor() as i64).max(1);
    let epsg = guide.epsg();
    for k in 0..=steps {
        if aborted() {
            return Err(Error::Aborted);
        }
        let s = (k as f64 * step).min(length);
        let Some((ge, gn, gb)) = guide.at(s) else { continue };
        let ahead = foresee(&points, s);
        let base = ahead.map(|a| a.1.atan() * 180.0 / std::f64::consts::PI).unwrap_or(0.0);
        let around = ahead.map(|a| a.0).unwrap_or(0.0);
        let mut tries: Vec<(f64, f64)> = Vec::new();
        if ahead.is_some() {
            tries.push((base, FOLLOW_WINDOW));
        }
        tries.extend(SEARCH_TURNS.iter().map(|t| (base + t, guide.window())));
        let mut det = None;
        let mut turn = 0.0;
        for (t, window) in tries {
            turn = t;
            let d = detect((ge, gn), gb + t, &epsg, around, window)?;
            let found = matches!(d, Detection::Found { .. });
            det = Some(d);
            if found {
                break;
            }
        }
        match det {
            Some(Detection::Found {
                left,
                right,
                axis,
                gauge,
                cant,
                good,
                ..
            }) => {
                let rad = (gb + turn) * DEG;
                let easting = ge + rad.cos() * axis;
                let northing = gn - rad.sin() * axis;
                let g_rad = gb * DEG;
                points.push(Point {
                    station: s,
                    easting,
                    northing,
                    offset: (easting - ge) * g_rad.cos() - (northing - gn) * g_rad.sin(),
                    turn,
                    z_left: left.z,
                    z_right: right.z,
                    cant,
                    gauge,
                    quality: if good { "good" } else { "doubtful" }.into(),
                });
            }
            _ => missed.push(s),
        }
        on_progress(k as f64 / steps as f64);
    }
    let n = rail_across_js::number;
    Ok(json!({
        "points": points.iter().map(Point::to_json).collect::<Vec<_>>(),
        "gaps": gaps_of(&missed, step).into_iter().map(|(f, t)| json!({ "from": n(f), "to": n(t) })).collect::<Vec<_>>(),
        "epsg": epsg,
    }))
}

/// The height of the rail top an axis point stands for (`soHeight`).
pub fn so_height(p: &Point, reference: &str) -> f64 {
    let (zl, zr) = (p.z_left, p.z_right);
    match reference {
        "axis" => (zl + zr) / 2.0,
        "left" => zl,
        "right" => zr,
        _ => zl.min(zr),
    }
}

/// The cant of a point [mm] (`cantMm`).
pub fn cant_mm(p: &Point) -> f64 {
    if p.cant.abs() < 0.003 { 0.0 } else { js_round(p.cant * 1000.0) }
}

pub const AXIS_CSV_HEADER: &str = "Nr;Station [m];Rechtswert [m];Hochwert [m];SO [m];Überhöhung [mm];Güte";

/// The traced points as a point file (`axisPointsCsv`).
pub fn axis_points_csv(points: &[Point], so_reference: &str) -> String {
    let mut lines = vec![AXIS_CSV_HEADER.to_string()];
    for (i, p) in points.iter().enumerate() {
        let quality = match p.quality.as_str() {
            "good" => "gut",
            "doubtful" => "fraglich",
            q => q,
        };
        lines.push(format!(
            "{};{};{};{};{};{};{}",
            i + 1,
            js_to_fixed(p.station, 3),
            js_to_fixed(p.easting, 4),
            js_to_fixed(p.northing, 4),
            js_to_fixed(so_height(p, so_reference), 4),
            rail_across_js::js_string(Some(&rail_across_js::number(cant_mm(p)))),
            quality
        ));
    }
    lines.join("\r\n") + "\r\n"
}
