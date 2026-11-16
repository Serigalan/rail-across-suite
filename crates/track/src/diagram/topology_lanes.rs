// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The layout of the topology diagram: every line through the network on a
//! row of its own (`topologyLanes.js`).

use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap};

use rail_across_js::{js_string, number};
use serde_json::{Map, Value, json};

const SIDE_DISTANCE: f64 = 60.0;
const MIN_SPAN: f64 = 2.0;
const MAX_SPAN: f64 = 6.0;

fn port_side(kind: &str, port: &str) -> Option<usize> {
    match kind {
        "switch" => match port {
            "A" => Some(0),
            "B1" | "B2" => Some(1),
            _ => None,
        },
        "crossing" | "single_slip" | "double_slip" => match port {
            "A" | "B" => Some(0),
            "C" | "D" => Some(1),
            _ => None,
        },
        "link" => match port {
            "A" => Some(0),
            "B" => Some(1),
            _ => None,
        },
        "stage_link" => match port {
            "A" => Some(0),
            "B1" | "B2" => Some(1),
            _ => None,
        },
        _ => None,
    }
}

fn has_sides(kind: &str) -> bool {
    matches!(kind, "switch" | "crossing" | "single_slip" | "double_slip" | "link" | "stage_link")
}

fn through(kind: &str) -> &'static [(&'static str, &'static str)] {
    match kind {
        "switch" => &[("A", "B2")],
        "crossing" | "single_slip" | "double_slip" => &[("A", "C")],
        "link" => &[("A", "B")],
        "stage_link" => &[("A", "B1")],
        _ => &[],
    }
}

/// A comparator's number as an order, NaN as equal (as `Array.prototype.sort` takes it).
fn by(x: f64) -> Ordering {
    if x < 0.0 {
        Ordering::Less
    } else if x > 0.0 {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}

type P = [f64; 2];

fn local([lng, lat]: P, [lng0, lat0]: P) -> P {
    [(lng - lng0) * 111320.0 * (lat0 * std::f64::consts::PI / 180.0).cos(), (lat - lat0) * 110574.0]
}

fn cross(a: P, b: P) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

fn minus(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1]]
}

fn along_axis(pts: &[P]) -> Vec<f64> {
    let n = pts.len() as f64;
    let mx = pts.iter().map(|p| p[0]).sum::<f64>() / n;
    let my = pts.iter().map(|p| p[1]).sum::<f64>() / n;
    let (mut sxx, mut syy, mut sxy) = (0.0, 0.0, 0.0);
    for &[x, y] in pts {
        sxx += (x - mx).powi(2);
        syy += (y - my).powi(2);
        sxy += (x - mx) * (y - my);
    }
    let angle = 0.5 * (2.0 * sxy).atan2(sxx - syy);
    let mut ax = [angle.cos(), angle.sin()];
    if ax[0] < -1e-9 || (ax[0].abs() < 1e-9 && ax[1] < 0.0) {
        ax = [-ax[0], -ax[1]];
    }
    pts.iter().map(|&[x, y]| (x - mx) * ax[0] + (y - my) * ax[1]).collect()
}

fn line_length(line: &[P]) -> f64 {
    line.windows(2).map(|w| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1])).sum()
}

fn point_along(line: &[P], mut d: f64) -> P {
    for k in 1..line.len() {
        let step = (line[k][0] - line[k - 1][0]).hypot(line[k][1] - line[k - 1][1]);
        if step >= d && step > 0.0 {
            let t = d / step;
            return [
                line[k - 1][0] + t * (line[k][0] - line[k - 1][0]),
                line[k - 1][1] + t * (line[k][1] - line[k - 1][1]),
            ];
        }
        d -= step;
    }
    line[line.len() - 1]
}

fn distance_to_line(p: P, line: &[P]) -> f64 {
    if line.len() == 1 {
        return (p[0] - line[0][0]).hypot(p[1] - line[0][1]);
    }
    let mut best = f64::INFINITY;
    for k in 1..line.len() {
        let (a, b) = (line[k - 1], line[k]);
        let (ab, ap) = (minus(b, a), minus(p, a));
        let l2 = ab[0].powi(2) + ab[1].powi(2);
        let t = if l2 != 0.0 {
            0f64.max(1f64.min((ap[0] * ab[0] + ap[1] * ab[1]) / l2))
        } else {
            0.0
        };
        best = best.min((ap[0] - t * ab[0]).hypot(ap[1] - t * ab[1]));
    }
    best
}

struct Parity {
    parent: Vec<usize>,
    parity: Vec<u8>,
}

impl Parity {
    fn new(n: usize) -> Parity {
        Parity {
            parent: (0..n).collect(),
            parity: vec![0; n],
        }
    }
    fn find(&mut self, x: usize) -> (usize, u8) {
        let (mut p, mut r) = (0u8, x);
        while self.parent[r] != r {
            p ^= self.parity[r];
            r = self.parent[r];
        }
        let (mut q, mut pq) = (x, p);
        while self.parent[q] != r {
            let (next, pn) = (self.parent[q], pq ^ self.parity[q]);
            self.parent[q] = r;
            self.parity[q] = pq;
            q = next;
            pq = pn;
        }
        (r, p)
    }
    fn join(&mut self, a: usize, b: usize, d: u8) -> bool {
        let ((ra, pa), (rb, pb)) = (self.find(a), self.find(b));
        if ra == rb {
            return (pa ^ pb) == d;
        }
        self.parent[ra] = rb;
        self.parity[ra] = pa ^ pb ^ d;
        true
    }
}

struct Edge {
    track_id: Value,
    from_port: Option<String>,
    to_port: Option<String>,
    u: usize,
    v: usize,
    line: Vec<P>,
    len: f64,
    weight: f64,
}

#[derive(Clone, Copy)]
struct End {
    k: usize,
    end: usize,
}

struct Line {
    edges: Vec<usize>,
    nodes: Vec<usize>,
    weight: f64,
}

fn pt(v: &Value) -> P {
    [
        v.get(0).and_then(Value::as_f64).unwrap_or(f64::NAN),
        v.get(1).and_then(Value::as_f64).unwrap_or(f64::NAN),
    ]
}

/// A layout of one cluster (`layoutClusterLanes`): { pos, routes, labels }.
pub fn layout_cluster_lanes(cluster: &Value) -> Value {
    let nodes = cluster.get("nodes").and_then(Value::as_array).cloned().unwrap_or_default();
    let n_nodes = nodes.len();
    if n_nodes == 0 {
        return json!({ "pos": {}, "routes": {}, "labels": {} });
    }
    let index: HashMap<String, usize> = nodes.iter().enumerate().map(|(i, n)| (js_string(n.get("id")), i)).collect();
    let origin = pt(&nodes[0]["lngLat"]);
    let xy: Vec<P> = nodes.iter().map(|n| local(pt(&n["lngLat"]), origin)).collect();
    let along = along_axis(&xy);
    let text = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
    let kind_of = |v: usize| text(&nodes[v], "kind").unwrap_or_default();
    let switch_kind = |v: usize| text(&nodes[v], "switchKind").unwrap_or_default();

    let edges: Vec<Edge> = cluster
        .get("edges")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|e| e.get("from") != e.get("to"))
        .map(|e| {
            let u = index.get(&js_string(e.get("from"))).copied().unwrap_or(0);
            let v = index.get(&js_string(e.get("to"))).copied().unwrap_or(0);
            let coords = e.get("coords").and_then(Value::as_array).filter(|c| c.len() >= 2);
            let line: Vec<P> = match coords {
                Some(c) => c.iter().map(|c| local(pt(c), origin)).collect(),
                None => vec![xy[u], xy[v]],
            };
            let len = line_length(&line);
            let length = e.get("length").and_then(Value::as_f64).unwrap_or(0.0);
            Edge {
                track_id: e.get("trackId").cloned().unwrap_or(Value::Null),
                from_port: text(e, "fromPort"),
                to_port: text(e, "toPort"),
                u,
                v,
                len,
                weight: if length != 0.0 && !length.is_nan() { length } else { len },
                line,
            }
        })
        .collect();
    let kk = edges.len();
    let mut ends: Vec<Vec<(End, Option<String>)>> = vec![Vec::new(); n_nodes];
    for (k, ed) in edges.iter().enumerate() {
        ends[ed.u].push((End { k, end: 0 }, ed.from_port.clone()));
        ends[ed.v].push((End { k, end: 1 }, ed.to_port.clone()));
    }
    let mut side_of: Vec<Option<usize>> = vec![None; 2 * kk];
    for (v, at_v) in ends.iter().enumerate() {
        let kind = kind_of(v);
        let sk = switch_kind(v);
        if (kind == "switch" || kind == "link") && has_sides(&sk) {
            for (at, port) in at_v {
                side_of[at.k * 2 + at.end] = port.as_deref().and_then(|p| port_side(&sk, p));
            }
        } else if kind == "joint" && at_v.len() == 2 {
            for (j, (at, _)) in at_v.iter().enumerate() {
                side_of[at.k * 2 + at.end] = Some(j);
            }
        }
    }

    let mut sets = Parity::new(kk + n_nodes);
    let mut bent = vec![false; kk];
    let mut by_weight: Vec<usize> = (0..kk).collect();
    by_weight.sort_by(|&a, &b| by(edges[b].weight - edges[a].weight).then(a.cmp(&b)));
    for &k in &by_weight {
        for end in [0usize, 1] {
            let Some(side) = side_of[k * 2 + end] else { continue };
            let v = if end == 1 { edges[k].v } else { edges[k].u };
            if !sets.join(k, kk + v, (side ^ end) as u8) {
                bent[k] = true;
            }
        }
    }
    let mut lean: HashMap<usize, f64> = HashMap::new();
    for (k, ed) in edges.iter().enumerate() {
        if bent[k] {
            continue;
        }
        let (root, p) = sets.find(k);
        *lean.entry(root).or_insert(0.0) += (along[ed.v] - along[ed.u]) * if p != 0 { 1.0 } else { -1.0 };
    }
    let mut value = |x: usize| -> u8 {
        let (root, p) = sets.find(x);
        p ^ u8::from(lean.get(&root).copied().unwrap_or(0.0) < 0.0)
    };
    let mut left_of = vec![0usize; kk];
    let mut right_of = vec![0usize; kk];
    for (k, ed) in edges.iter().enumerate() {
        let forward = value(k) == 1;
        left_of[k] = if forward { ed.u } else { ed.v };
        right_of[k] = if forward { ed.v } else { ed.u };
    }
    let node_turn: Vec<u8> = (0..n_nodes).map(|v| value(kk + v)).collect();
    let leaves_right = |k: usize, v: usize| left_of[k] == v;
    let bent_side = |k: usize, end: usize| {
        let v = if end == 1 { edges[k].v } else { edges[k].u };
        side_of[k * 2 + end].unwrap_or(0) as u8 ^ node_turn[v]
    };

    let mut into: Vec<Vec<usize>> = vec![Vec::new(); n_nodes];
    let mut out_of: Vec<Vec<usize>> = vec![Vec::new(); n_nodes];
    let mut indeg = vec![0i64; n_nodes];
    for k in 0..kk {
        if bent[k] {
            continue;
        }
        out_of[left_of[k]].push(k);
        into[right_of[k]].push(k);
        indeg[right_of[k]] += 1;
    }
    let mut order: Vec<usize> = Vec::new();
    let mut done = vec![false; n_nodes];
    let mut ready: Vec<usize> = (0..n_nodes).filter(|&v| indeg[v] == 0).collect();
    while order.len() < n_nodes {
        if ready.is_empty() {
            let mut v: Option<usize> = None;
            for w in 0..n_nodes {
                if !done[w] && v.is_none_or(|v| along[w] < along[v]) {
                    v = Some(w);
                }
            }
            let v = v.unwrap_or(0);
            for &k in &into[v] {
                if !bent[k] {
                    bent[k] = true;
                    indeg[v] -= 1;
                }
            }
            ready.push(v);
            continue;
        }
        ready.sort_by(|&a, &b| by(along[b] - along[a]));
        let v = ready.pop().unwrap_or(0);
        if done[v] {
            continue;
        }
        done[v] = true;
        order.push(v);
        for &k in &out_of[v] {
            if bent[k] {
                continue;
            }
            indeg[right_of[k]] -= 1;
            if indeg[right_of[k]] == 0 {
                ready.push(right_of[k]);
            }
        }
    }
    let columns = |span_of: &dyn Fn(usize) -> f64| -> Vec<f64> {
        let mut x = vec![0.0f64; n_nodes];
        for &v in &order {
            for &k in &into[v] {
                if !bent[k] {
                    x[v] = x[v].max(x[left_of[k]] + span_of(k));
                }
            }
        }
        for pass in 0..100 {
            let mut moved = false;
            let seq: Vec<usize> = if pass % 2 == 1 {
                order.iter().rev().copied().collect()
            } else {
                order.clone()
            };
            for v in seq {
                let (mut lo, mut hi, mut ins, mut outs) = (f64::NEG_INFINITY, f64::INFINITY, 0, 0);
                for &k in &into[v] {
                    if !bent[k] {
                        lo = lo.max(x[left_of[k]] + span_of(k));
                        ins += 1;
                    }
                }
                for &k in &out_of[v] {
                    if !bent[k] {
                        hi = hi.min(x[right_of[k]] - span_of(k));
                        outs += 1;
                    }
                }
                let want = if ins > outs {
                    lo
                } else if outs > ins {
                    hi
                } else {
                    x[v]
                };
                if want.is_finite() && want != x[v] {
                    x[v] = want;
                    moved = true;
                }
            }
            if !moved {
                break;
            }
        }
        let x0 = x.iter().copied().fold(f64::INFINITY, f64::min);
        x.iter().map(|v| v - x0).collect()
    };

    let mut mate = vec![-1i64; 2 * kk];
    for v in 0..n_nodes {
        let kind = kind_of(v);
        let mut pairs: Vec<(End, End)> = Vec::new();
        if kind == "switch" || kind == "link" {
            let lp = text(&nodes[v], "linePort").filter(|s| !s.is_empty());
            let thr: Vec<(String, String)> = match lp {
                Some(lp) => vec![("A".into(), lp)],
                None => through(&switch_kind(v)).iter().map(|(a, b)| (a.to_string(), b.to_string())).collect(),
            };
            for (p, q) in thr {
                let a = ends[v].iter().find(|(_, port)| port.as_deref() == Some(p.as_str()));
                let b = ends[v].iter().find(|(_, port)| port.as_deref() == Some(q.as_str()));
                if let (Some(a), Some(b)) = (a, b) {
                    pairs.push((a.0, b.0));
                }
            }
        } else if kind == "joint" && ends[v].len() == 2 {
            pairs.push((ends[v][0].0, ends[v][1].0));
        }
        for (a, b) in pairs {
            if bent[a.k] || bent[b.k] || a.k == b.k {
                continue;
            }
            mate[a.k * 2 + a.end] = (b.k * 2 + b.end) as i64;
            mate[b.k * 2 + b.end] = (a.k * 2 + a.end) as i64;
        }
    }
    let end_at = |k: usize, v: usize| usize::from(edges[k].u != v);
    let mut line_of_edge = vec![-1i64; kk];
    let mut lines: Vec<Line> = Vec::new();
    for &k0 in &by_weight {
        if bent[k0] || line_of_edge[k0] >= 0 {
            continue;
        }
        let mut k = k0;
        let mut seen = std::collections::HashSet::from([k]);
        loop {
            let m = mate[k * 2 + end_at(k, left_of[k])];
            if m < 0 || seen.contains(&((m >> 1) as usize)) {
                break;
            }
            k = (m >> 1) as usize;
            seen.insert(k);
        }
        let mut line = Line {
            edges: vec![],
            nodes: vec![left_of[k]],
            weight: 0.0,
        };
        loop {
            line.edges.push(k);
            line.nodes.push(right_of[k]);
            line.weight += edges[k].weight;
            line_of_edge[k] = lines.len() as i64;
            let m = mate[k * 2 + end_at(k, right_of[k])];
            if m < 0 || line_of_edge[(m >> 1) as usize] >= 0 {
                break;
            }
            k = (m >> 1) as usize;
        }
        lines.push(line);
    }
    let mut owner = vec![usize::MAX; n_nodes];
    for (s, ln) in lines.iter().enumerate() {
        for (j, &v) in ln.nodes.iter().enumerate() {
            if j > 0 && j < ln.nodes.len() - 1 {
                owner[v] = s;
            }
        }
    }
    for v in 0..n_nodes {
        if owner[v] != usize::MAX {
            continue;
        }
        let mut best: Option<usize> = None;
        for (at, _) in &ends[v] {
            let s = line_of_edge[at.k];
            if s >= 0 && best.is_none_or(|b| lines[s as usize].weight > lines[b].weight) {
                best = Some(s as usize);
            }
        }
        let best = best.unwrap_or_else(|| {
            lines.push(Line {
                edges: vec![],
                nodes: vec![v],
                weight: 0.0,
            });
            lines.len() - 1
        });
        owner[v] = best;
    }

    let ns = lines.len();
    let mut owns: Vec<Vec<usize>> = vec![Vec::new(); ns];
    for v in 0..n_nodes {
        owns[owner[v]].push(v);
    }
    let is_crossover: Vec<bool> = lines
        .iter()
        .enumerate()
        .map(|(s, ln)| owns[s].is_empty() && owner[ln.nodes[0]] != owner[ln.nodes[ln.nodes.len() - 1]])
        .collect();
    let point_from = |k: usize, v: usize, d: f64| {
        let ed = &edges[k];
        if ed.u == v {
            point_along(&ed.line, d)
        } else {
            let rev: Vec<P> = ed.line.iter().rev().copied().collect();
            point_along(&rev, d)
        }
    };
    let side_at = |v: usize, k: usize, s: usize| -> f64 {
        let right = leaves_right(k, v);
        let own: Vec<usize> = lines[s]
            .edges
            .iter()
            .copied()
            .filter(|&j| j != k && (edges[j].u == v || edges[j].v == v))
            .collect();
        let same = own.iter().copied().find(|&j| leaves_right(j, v) == right);
        let other = own.iter().copied().find(|&j| leaves_right(j, v) != right);
        let Some(j) = same.or(other) else { return 0.0 };
        let d = SIDE_DISTANCE.min(edges[k].len).min(edges[j].len);
        if !(d > 0.0) {
            return 0.0;
        }
        let p = xy[v];
        let mut dir = minus(point_from(j, v, d), p);
        if same.is_none() {
            dir = [-dir[0], -dir[1]];
        }
        let c = cross(dir, minus(point_from(k, v, d), p));
        js_sign(c) * if right { 1.0 } else { -1.0 }
    };

    // rel[a]: (b, how far a lies above b), in the order first told.
    let mut rel: Vec<Vec<(usize, f64)>> = vec![Vec::new(); ns];
    let tell = |rel: &mut Vec<Vec<(usize, f64)>>, a: usize, b: usize, up: f64| {
        if a == b || up == 0.0 || up.is_nan() {
            return;
        }
        for (x, y, d) in [(a, b, up), (b, a, -up)] {
            match rel[x].iter_mut().find(|e| e.0 == y) {
                Some(e) => e.1 += d,
                None => rel[x].push((y, d)),
            }
        }
    };
    for v in 0..n_nodes {
        let s = owner[v];
        for (at, _) in &ends[v] {
            let t = line_of_edge[at.k];
            if t < 0 || t as usize == s {
                continue;
            }
            let t = t as usize;
            let up = side_at(v, at.k, s);
            if !is_crossover[t] {
                tell(&mut rel, t, s, up);
                continue;
            }
            let ln = &lines[t];
            let w = if ln.nodes[0] == v { ln.nodes[ln.nodes.len() - 1] } else { ln.nodes[0] };
            tell(&mut rel, owner[w], s, up);
        }
    }
    let relation = |a: usize, b: usize| rel[a].iter().find(|e| e.0 == b).map(|e| e.1).unwrap_or(0.0);
    let measure = |a: usize, b: usize| -> f64 {
        let pts: Vec<P> = (if owns[a].is_empty() { &lines[a].nodes } else { &owns[a] }).iter().map(|&v| xy[v]).collect();
        let mut tracks: Vec<Vec<P>> = lines[b].edges.iter().map(|&k| edges[k].line.clone()).collect();
        if tracks.is_empty() {
            tracks.push(owns[b].iter().map(|&v| xy[v]).collect());
        }
        let mut ds: Vec<f64> = pts
            .iter()
            .map(|&p| tracks.iter().map(|l| distance_to_line(p, l)).fold(f64::INFINITY, f64::min))
            .collect();
        ds.sort_by(|p, q| by(p - q));
        ds[ds.len() / 2]
    };
    let mut distances: HashMap<usize, f64> = HashMap::new();
    let mut distance = |a: usize, b: usize| *distances.entry(a * ns + b).or_insert_with(|| measure(a, b));

    let mut place_rows = |x: &[f64]| -> Vec<f64> {
        let span: Vec<(f64, f64)> = lines
            .iter()
            .enumerate()
            .map(|(s, ln)| {
                let (first, last) = (ln.nodes[0], ln.nodes[ln.nodes.len() - 1]);
                let mut lo = if owner[first] == s { x[first] } else { x[first] + 1.0 };
                let mut hi = if owner[last] == s { x[last] } else { x[last] - 1.0 };
                for &v in &owns[s] {
                    lo = lo.min(x[v]);
                    hi = hi.max(x[v]);
                }
                (lo, hi)
            })
            .collect();
        // Rows by id; `rows` their order bottom to top.
        let mut rows: Vec<usize> = Vec::new();
        let mut members: Vec<Vec<usize>> = Vec::new();
        let mut row_of: Vec<Option<usize>> = vec![None; ns];
        let overlaps = |a: usize, b: usize| span[a].0 <= span[b].1 && span[b].0 <= span[a].1;
        let mut lane_lines: Vec<usize> = (0..ns).filter(|&s| !is_crossover[s]).collect();
        lane_lines.sort_by(|&a, &b| by(lines[b].weight - lines[a].weight).then(a.cmp(&b)));
        let new_row = |members: &mut Vec<Vec<usize>>| {
            members.push(Vec::new());
            members.len() - 1
        };
        let mut place_near = |s: usize, rows: &mut Vec<usize>, members: &mut Vec<Vec<usize>>, row_of: &mut Vec<Option<usize>>| {
            let (mut anchor, mut best): (Option<usize>, f64) = (None, 0.0);
            for &(b, r) in &rel[s] {
                if row_of[b].is_none() {
                    continue;
                }
                let score = r.abs() * 1e9 + lines[b].weight;
                if anchor.is_none() || score > best {
                    anchor = Some(b);
                    best = score;
                }
            }
            let Some(anchor) = anchor else {
                let row = new_row(members);
                rows.push(row);
                members[row].push(s);
                row_of[s] = Some(row);
                return;
            };
            let sg = js_sign(relation(s, anchor));
            let dir: i64 = if sg == 0.0 || sg.is_nan() { 1 } else { sg as i64 };
            let mut j = rows.iter().position(|&r| Some(r) == row_of[anchor]).map(|p| p as i64).unwrap_or(-1) + dir;
            loop {
                if j < 0 || j >= rows.len() as i64 {
                    let row = new_row(members);
                    if j < 0 {
                        rows.insert(0, row);
                    } else {
                        rows.push(row);
                    }
                    members[row].push(s);
                    row_of[s] = Some(row);
                    return;
                }
                let in_way: Vec<usize> = members[rows[j as usize]].iter().copied().filter(|&q| overlaps(q, s)).collect();
                if in_way.is_empty() {
                    let row = rows[j as usize];
                    members[row].push(s);
                    row_of[s] = Some(row);
                    return;
                }
                let q = in_way.iter().copied().find(|&q| relation(s, q) != 0.0).unwrap_or(in_way[0]);
                let r = relation(s, q);
                let before = if r != 0.0 {
                    js_sign(r) as i64 == -dir
                } else {
                    distance(s, anchor) < distance(q, anchor)
                };
                if before {
                    let row = new_row(members);
                    rows.insert(if dir > 0 { j as usize } else { j as usize + 1 }, row);
                    members[row].push(s);
                    row_of[s] = Some(row);
                    return;
                }
                j += dir;
            }
        };
        for &first in &lane_lines {
            if row_of[first].is_some() {
                continue;
            }
            place_near(first, &mut rows, &mut members, &mut row_of);
            let mut waiting: BTreeSet<usize> = BTreeSet::new();
            for &(b, _) in &rel[first] {
                if row_of[b].is_none() && !is_crossover[b] {
                    waiting.insert(b);
                }
            }
            while !waiting.is_empty() {
                let mut s: Option<usize> = None;
                for &c in &waiting {
                    if s.is_none_or(|s| lines[c].weight > lines[s].weight || (lines[c].weight == lines[s].weight && c < s)) {
                        s = Some(c);
                    }
                }
                let s = s.unwrap_or(0);
                waiting.remove(&s);
                place_near(s, &mut rows, &mut members, &mut row_of);
                for &(b, _) in &rel[s] {
                    if row_of[b].is_none() && !is_crossover[b] {
                        waiting.insert(b);
                    }
                }
            }
        }
        let main_row = lane_lines
            .first()
            .and_then(|&l| rows.iter().position(|&r| Some(r) == row_of[l]))
            .map(|p| p as f64)
            .unwrap_or(-1.0);
        row_of
            .iter()
            .map(|r| {
                r.map(|r| rows.iter().position(|&x| x == r).map(|p| p as f64 - main_row).unwrap_or(f64::NAN))
                    .unwrap_or(0.0)
            })
            .collect()
    };

    let mut x = columns(&|_| MIN_SPAN);
    let mut y_of = place_rows(&x);
    let climb = |k: usize, y_of: &[f64]| -> f64 {
        let s = line_of_edge[k] as usize;
        let (ya, yb) = (y_of[owner[left_of[k]]], y_of[owner[right_of[k]]]);
        if is_crossover[s] {
            return (ya - yb).abs();
        }
        let d = (ya - y_of[s]).abs() + (yb - y_of[s]).abs();
        if d != 0.0 { d + 1.0 } else { 0.0 }
    };
    let spans: Vec<f64> = (0..kk)
        .map(|k| if bent[k] { MIN_SPAN } else { MAX_SPAN.min(MIN_SPAN.max(climb(k, &y_of))) })
        .collect();
    if spans.iter().any(|&sp| sp > MIN_SPAN) {
        x = columns(&|k| spans[k]);
        y_of = place_rows(&x);
    }
    let at = |v: usize| (x[v], y_of[owner[v]]);
    let xyj = |p: (f64, f64)| json!({ "x": number(p.0), "y": number(p.1) });
    let pos: Map<String, Value> = nodes.iter().enumerate().map(|(v, n)| (js_string(n.get("id")), xyj(at(v)))).collect();
    let mut routes = Map::new();
    let mut labels = Map::new();
    for (k, ed) in edges.iter().enumerate() {
        let id = js_string(Some(&ed.track_id));
        let (a, b) = (left_of[k], right_of[k]);
        let (pa, pb) = (at(a), at(b));
        if bent[k] {
            let right = bent_side(k, end_at(k, a)) == 1;
            let turn = if right { pa.0.max(pb.0) + 1.5 } else { pa.0.min(pb.0) - 1.5 };
            let mid = (turn, if pa.1 == pb.1 { pa.1 + 0.5 } else { (pa.1 + pb.1) / 2.0 });
            routes.insert(id.clone(), json!([xyj(pa), xyj(mid), xyj(pb)]));
            labels.insert(id, xyj(mid));
            continue;
        }
        let s = line_of_edge[k] as usize;
        if is_crossover[s] {
            if pa.1 == pb.1 {
                routes.insert(
                    id.clone(),
                    json!([xyj(pa), xyj((pa.0 + 1.0, pa.1 + 0.5)), xyj((pb.0 - 1.0, pb.1 + 0.5)), xyj(pb)]),
                );
                labels.insert(id, xyj(((pa.0 + pb.0) / 2.0, pa.1 + 0.5)));
            } else {
                routes.insert(id.clone(), json!([xyj(pa), xyj(pb)]));
                labels.insert(id, xyj(((pa.0 + pb.0) / 2.0, (pa.1 + pb.1) / 2.0)));
            }
            continue;
        }
        let y = y_of[s];
        let (dl, dr) = ((pa.1 - y).abs(), (pb.1 - y).abs());
        let room = pb.0 - pa.0;
        let wl = if dl != 0.0 { dl.min(1f64.max((room * dl / (dl + dr)).floor())) } else { 0.0 };
        let wr = if dr != 0.0 { dr.min(1f64.max(room - wl)) } else { 0.0 };
        let (x1, x2) = (pa.0 + wl, pb.0 - wr);
        let all = [pa, (x1, y), (x2, y), pb];
        let route: Vec<Value> = all
            .iter()
            .enumerate()
            .filter(|(j, p)| *j == 0 || p.0 != all[j - 1].0 || p.1 != all[j - 1].1)
            .map(|(_, p)| xyj(*p))
            .collect();
        routes.insert(id.clone(), Value::Array(route));
        labels.insert(id, xyj(((x1 + x2) / 2.0, y)));
    }
    json!({ "pos": pos, "routes": routes, "labels": labels })
}

/// `Math.sign`: −1, 0 or 1, NaN for NaN.
fn js_sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        x
    }
}
