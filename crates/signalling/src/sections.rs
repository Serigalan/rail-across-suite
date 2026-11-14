// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The extent of the track detection sections over the net (`trackSections.js`).

use std::collections::{HashMap, HashSet};

use rail_across_js::{cmp_utf16, js_string};
use rail_across_track::topology::tracks_on_from;
use serde_json::{Value, json};

fn length_of(track: &Value) -> f64 {
    track
        .get("elements")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|e| rail_across_js::get(e, "length").and_then(Value::as_f64).unwrap_or(0.0))
        .sum()
}

fn coll<'a>(domain: &'a Value, name: &str) -> &'a [Value] {
    domain
        .get("collections")
        .and_then(|c| c.get(name))
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

struct UnionFind {
    parent: HashMap<String, String>,
}

impl UnionFind {
    fn find(&mut self, k: &str) -> String {
        if !self.parent.contains_key(k) {
            self.parent.insert(k.to_string(), k.to_string());
        }
        let mut r = k.to_string();
        while self.parent[&r] != r {
            r = self.parent[&r].clone();
        }
        let mut k = k.to_string();
        while self.parent[&k] != r {
            let n = self.parent[&k].clone();
            self.parent.insert(k, r.clone());
            k = n;
        }
        r
    }
    fn union(&mut self, a: &str, b: &str) {
        let (ra, rb) = (self.find(a), self.find(b));
        self.parent.insert(ra, rb);
    }
}

/// A part of the net between detection points.
#[derive(Clone, Debug)]
pub struct Part {
    pub parts: Vec<(Value, f64, f64)>,
    pub boundary: Vec<String>,
}

impl Part {
    pub fn to_json(&self) -> Value {
        json!({
            "parts": self.parts.iter().map(|(t, f, to)| json!({ "trackId": t, "from": rail_across_js::number(*f), "to": rail_across_js::number(*to) })).collect::<Vec<_>>(),
            "boundary": self.boundary,
        })
    }
}

/// The parts the detection points cut the net into (`netParts`).
pub fn net_parts(core: &Value, signalling: &Value, only: Option<&HashSet<String>>) -> Vec<Part> {
    let tracks = coll(core, "tracks");
    let switches = coll(core, "switches");
    let mut cuts: HashMap<String, Vec<(f64, String)>> = HashMap::new();
    for d in coll(signalling, "detectionPoints") {
        let at = &d["at"];
        let id = js_string(d.get("id"));
        if at.get("anchor").and_then(Value::as_str) != Some("station") || only.is_some_and(|o| !o.contains(&id)) {
            continue;
        }
        cuts.entry(js_string(at.get("trackId")))
            .or_default()
            .push((at.get("station").and_then(Value::as_f64).unwrap_or(f64::NAN), id));
    }
    for list in cuts.values_mut() {
        list.sort_by(|a, b| (a.0 - b.0).partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal));
    }
    struct Piece {
        track_id: Value,
        from: f64,
        to: f64,
        bounds: Vec<String>,
    }
    let mut pieces: Vec<(String, Piece)> = Vec::new();
    let mut count: HashMap<String, usize> = HashMap::new();
    let mut uf = UnionFind { parent: HashMap::new() };
    for t in tracks {
        let tid = js_string(t.get("id"));
        let length = length_of(t);
        let list: Vec<(f64, String)> = cuts
            .get(&tid)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|c| c.0 >= 0.0 && c.0 <= length)
            .collect();
        let mut edges: Vec<(f64, Option<String>)> = vec![(0.0, None)];
        edges.extend(list.into_iter().map(|(s, id)| (s, Some(id))));
        edges.push((length, None));
        count.insert(tid.clone(), edges.len() - 1);
        for i in 0..edges.len() - 1 {
            let key = format!("{tid}:{i}");
            let bounds = [&edges[i].1, &edges[i + 1].1].into_iter().flatten().cloned().collect();
            uf.find(&key);
            pieces.push((
                key,
                Piece {
                    track_id: t.get("id").cloned().unwrap_or(Value::Null),
                    from: edges[i].0,
                    to: edges[i + 1].0,
                    bounds,
                },
            ));
        }
    }
    let piece_at = |tid: &str, endpoint: &str| -> Option<String> {
        let n = *count.get(tid)?;
        (n > 0).then(|| format!("{tid}:{}", if endpoint == "BEGIN" { 0 } else { n - 1 }))
    };
    for t in tracks {
        let tid = js_string(t.get("id"));
        for endpoint in ["BEGIN", "END"] {
            let Some(here) = piece_at(&tid, endpoint) else { continue };
            for (next, ep) in tracks_on_from(t.get("id").unwrap_or(&Value::Null), endpoint, tracks, switches) {
                if let Some(there) = piece_at(&js_string(Some(&next)), &ep) {
                    uf.union(&here, &there);
                }
            }
        }
    }
    let mut parts: Vec<(String, Part)> = Vec::new();
    for (key, piece) in &pieces {
        let root = uf.find(key);
        let idx = match parts.iter().position(|(r, _)| *r == root) {
            Some(i) => i,
            None => {
                parts.push((
                    root,
                    Part {
                        parts: vec![],
                        boundary: vec![],
                    },
                ));
                parts.len() - 1
            }
        };
        let part = &mut parts[idx].1;
        if piece.to - piece.from > 1e-6 {
            part.parts.push((piece.track_id.clone(), piece.from, piece.to));
        }
        for b in &piece.bounds {
            if !part.boundary.contains(b) {
                part.boundary.push(b.clone());
            }
        }
    }
    parts
        .into_iter()
        .map(|(_, mut p)| {
            p.boundary.sort_by(|a, b| cmp_utf16(a, b));
            p
        })
        .filter(|p| !p.parts.is_empty())
        .collect()
}

/// A section's extent: its parts, whether it matches, the points missing.
pub struct Extent {
    pub parts: Vec<(Value, f64, f64)>,
    pub matches: bool,
    pub missing: Vec<String>,
}

/// The extent of every stored track section (`trackSectionExtents`).
pub fn track_section_extents(core: &Value, signalling: &Value) -> Vec<(Value, Extent)> {
    let sections = coll(signalling, "trackSections");
    let mut points_of: HashMap<String, Vec<String>> = HashMap::new();
    for s in sections {
        for p in s.get("detectionPointIds").and_then(Value::as_array).into_iter().flatten() {
            points_of.entry(js_string(Some(p))).or_default().push(js_string(s.get("id")));
        }
    }
    let mut stations: HashMap<String, Vec<(f64, String)>> = HashMap::new();
    for d in coll(signalling, "detectionPoints") {
        let at = &d["at"];
        if at.get("anchor").and_then(Value::as_str) != Some("station") {
            continue;
        }
        stations
            .entry(js_string(at.get("trackId")))
            .or_default()
            .push((at.get("station").and_then(Value::as_f64).unwrap_or(f64::NAN), js_string(d.get("id"))));
    }
    let foreign = |part: &Part, own: &HashSet<String>| -> usize {
        let mut n = 0;
        for (tid, from, to) in &part.parts {
            for (st, id) in stations.get(&js_string(Some(tid))).into_iter().flatten() {
                if !own.contains(id) && points_of.contains_key(id) && *st > from + 1e-6 && *st < to - 1e-6 {
                    n += 1;
                }
            }
        }
        n
    };
    let length = |part: &Part| part.parts.iter().map(|(_, f, t)| t - f).sum::<f64>();
    let mut out = Vec::new();
    for s in sections {
        let own: HashSet<String> = s
            .get("detectionPointIds")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|v| js_string(Some(v)))
            .collect();
        let parts = net_parts(core, signalling, Some(&own));
        let mut whole: Vec<Part> = parts
            .iter()
            .filter(|p| p.boundary.len() == own.len() && p.boundary.iter().all(|b| own.contains(b)))
            .cloned()
            .collect();
        let id = s.get("id").cloned().unwrap_or(Value::Null);
        if !whole.is_empty() {
            whole.sort_by(|a, b| {
                (foreign(a, &own) as i64 - foreign(b, &own) as i64)
                    .cmp(&0)
                    .then_with(|| (length(a) - length(b)).partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal))
            });
            out.push((
                id,
                Extent {
                    parts: whole[0].parts.clone(),
                    matches: true,
                    missing: vec![],
                },
            ));
            continue;
        }
        let mut bounded: Vec<&Part> = parts.iter().filter(|p| !p.boundary.is_empty()).collect();
        bounded.sort_by_key(|p| std::cmp::Reverse(p.boundary.len()));
        let mut own_sorted: Vec<String> = own.iter().cloned().collect();
        own_sorted.sort_by(|a, b| cmp_utf16(a, b));
        let extent = match bounded.first() {
            Some(best) => Extent {
                parts: best.parts.clone(),
                matches: false,
                missing: own_sorted.into_iter().filter(|b| !best.boundary.contains(b)).collect(),
            },
            None => Extent {
                parts: vec![],
                matches: false,
                missing: own_sorted,
            },
        };
        out.push((id, extent));
    }
    out
}
