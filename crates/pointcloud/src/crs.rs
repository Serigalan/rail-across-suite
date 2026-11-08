// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Coordinate systems around a point cloud (`cloudCrs.js`): the conversion
//! between two planes replaced by an affine map per cell of 250 m — the exact
//! conversion of the cell's centre and its Jacobian by central differences —
//! and the chain a cloud's points are read through, re-referencing included.

use std::cell::RefCell;
use std::collections::HashMap;

use rail_across_geom::coords::{def_for, transform_plane_point};

use crate::registration::invert_matrix;
use crate::{Error, Result};

const CELL: f64 = 250.0;
const H: f64 = CELL / 2.0;

#[derive(Clone, Copy)]
struct Cell {
    ce: f64,
    cn: f64,
    e0: f64,
    n0: f64,
    dee: f64,
    dne: f64,
    den: f64,
    dnn: f64,
}

/// `(e, n) → (e', n')` from one plane into another (`planeMapper`).
pub struct PlaneMapper {
    codes: Option<(i64, i64)>,
    cells: RefCell<HashMap<(i64, i64), Cell>>,
}

impl PlaneMapper {
    pub fn new(from: i64, to: i64) -> Result<Self> {
        if from == to {
            return Ok(PlaneMapper {
                codes: None,
                cells: RefCell::new(HashMap::new()),
            });
        }
        def_for(from).map_err(|e| Error::msg(e.0))?;
        def_for(to).map_err(|e| Error::msg(e.0))?;
        Ok(PlaneMapper {
            codes: Some((from, to)),
            cells: RefCell::new(HashMap::new()),
        })
    }

    pub fn map(&self, e: f64, n: f64) -> (f64, f64) {
        let Some((from, to)) = self.codes else { return (e, n) };
        let exact = |e: f64, n: f64| transform_plane_point(e, n, from, to).unwrap_or((f64::NAN, f64::NAN));
        let key = ((e / CELL).floor() as i64, (n / CELL).floor() as i64);
        let c = *self.cells.borrow_mut().entry(key).or_insert_with(|| {
            let (ce, cn) = ((key.0 as f64 + 0.5) * CELL, (key.1 as f64 + 0.5) * CELL);
            let (e0, n0) = exact(ce, cn);
            let (ee1, ne1) = exact(ce + H, cn);
            let (ee2, ne2) = exact(ce - H, cn);
            let (en1, nn1) = exact(ce, cn + H);
            let (en2, nn2) = exact(ce, cn - H);
            Cell {
                ce,
                cn,
                e0,
                n0,
                dee: (ee1 - ee2) / CELL,
                dne: (ne1 - ne2) / CELL,
                den: (en1 - en2) / CELL,
                dnn: (nn1 - nn2) / CELL,
            }
        });
        let (de, dn) = (e - c.ce, n - c.cn);
        (c.e0 + c.dee * de + c.den * dn, c.n0 + c.dne * de + c.dnn * dn)
    }
}

/// A cloud's re-referencing: the matrix into the plane of `crs`.
#[derive(Clone, Debug)]
pub struct Transform {
    pub matrix: [f64; 16],
    pub crs: i64,
}

/// A chain `(e, n, z) → (e, n, z)` from a cloud's file into a plane, or back.
pub struct Chain {
    pre: Option<PlaneMapper>,
    matrix: Option<[f64; 16]>,
    post: Option<PlaneMapper>,
}

impl Chain {
    pub fn apply(&self, e: f64, n: f64, z: f64) -> (f64, f64, f64) {
        let (mut e, mut n) = (e, n);
        if let Some(p) = &self.pre {
            (e, n) = p.map(e, n);
        }
        let Some(m) = self.matrix else {
            return match &self.post {
                Some(p) => {
                    let (x, y) = p.map(e, n);
                    (x, y, z)
                }
                None => (e, n, z),
            };
        };
        let mut x = m[0] * e + m[1] * n + m[2] * z + m[3];
        let mut y = m[4] * e + m[5] * n + m[6] * z + m[7];
        let h = m[8] * e + m[9] * n + m[10] * z + m[11];
        if let Some(p) = &self.post {
            (x, y) = p.map(x, y);
        }
        (x, y, h)
    }

    /// Whether the chain changes the height (a re-referencing).
    pub fn has_height(&self) -> bool {
        self.matrix.is_some()
    }
}

/// The plane a cloud's points are read in (`cloudPlane`).
pub fn cloud_plane(crs: Option<i64>, transform: Option<&Transform>) -> Option<i64> {
    transform.map(|t| t.crs).or(crs)
}

/// The chain from a cloud's file into plane `crs` (`cloudToPlane`); none where nothing is to be done.
pub fn cloud_to_plane(cloud_crs: Option<i64>, transform: Option<&Transform>, crs: i64) -> Result<Option<Chain>> {
    let Some(t) = transform else {
        let from = cloud_crs.unwrap_or(-1);
        if from == crs {
            return Ok(None);
        }
        return Ok(Some(Chain {
            pre: None,
            matrix: None,
            post: Some(PlaneMapper::new(from, crs)?),
        }));
    };
    let pre = match cloud_crs {
        Some(c) if c != t.crs => Some(PlaneMapper::new(c, t.crs)?),
        _ => None,
    };
    let post = if t.crs != crs { Some(PlaneMapper::new(t.crs, crs)?) } else { None };
    Ok(Some(Chain {
        pre,
        matrix: Some(t.matrix),
        post,
    }))
}

/// The same chain the other way (`planeToCloud`).
pub fn plane_to_cloud(cloud_crs: Option<i64>, transform: Option<&Transform>, crs: i64) -> Result<Option<Chain>> {
    let Some(t) = transform else {
        let to = cloud_crs.unwrap_or(-1);
        if to == crs {
            return Ok(None);
        }
        return Ok(Some(Chain {
            pre: None,
            matrix: None,
            post: Some(PlaneMapper::new(crs, to)?),
        }));
    };
    let pre = if t.crs != crs { Some(PlaneMapper::new(crs, t.crs)?) } else { None };
    let post = match cloud_crs {
        Some(c) if c != t.crs => Some(PlaneMapper::new(t.crs, c)?),
        _ => None,
    };
    Ok(Some(Chain {
        pre,
        matrix: Some(invert_matrix(&t.matrix)),
        post,
    }))
}
