// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The part of proj4js (2.20) the core converts with, computed step for step
//! as proj4js computes it, so a point lands on the same double: the extended
//! transverse Mercator (`utm`, `tmerc` — Poder/Engsager), Cassini-Soldner, the
//! datum transformation (geocentric, 7-parameter Helmert in position-vector
//! convention, NTv2 grids) and the pipeline of `transform.js`, the detour
//! through WGS 84 included.

use std::collections::HashMap;
use std::f64::consts::PI;
use std::sync::{OnceLock, RwLock};

const D2R: f64 = 0.017_453_292_519_943_295;
const R2D: f64 = 57.295_779_513_082_32;
/// proj4js's SPI: a hair more than π, so a longitude that drifted past ±180° by float noise is not wrapped.
#[allow(clippy::approx_constant)]
const SPI: f64 = 3.14159265359;
const TWO_PI: f64 = PI * 2.0;
const HALF_PI: f64 = PI / 2.0;
const EPSLN: f64 = 1.0e-10;
const SEC_TO_RAD: f64 = 4.848_136_811_095_36e-6;
const SRS_WGS84_SEMIMAJOR: f64 = 6_378_137.0;
const SRS_WGS84_SEMIMINOR: f64 = 6_356_752.314;
const SRS_WGS84_ESQUARED: f64 = 0.006_694_379_990_141_316_5;

/// The ellipsoids the planes are computed on: semi-major axis, inverse flattening.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ellps {
    Wgs84,
    Bessel,
    Krass,
}

impl Ellps {
    fn a_rf(self) -> (f64, f64) {
        match self {
            Ellps::Wgs84 => (6_378_137.0, 298.257_223_563),
            Ellps::Bessel => (6_377_397.155, 299.152_812_8),
            Ellps::Krass => (6_378_245.0, 298.3),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum DatumType {
    ThreeParam,
    SevenParam,
    GridShift,
    Wgs84,
    NoDatum,
}

/// A grid of a `+nadgrids` list: its key, whether it must be there, the grid if loaded.
#[derive(Clone, Debug)]
struct GridRef {
    name: String,
    mandatory: bool,
}

#[derive(Clone, Debug)]
struct Datum {
    kind: DatumType,
    params: Vec<f64>,
    grids: Vec<GridRef>,
    a: f64,
    b: f64,
    es: f64,
}

/// One subgrid of an NTv2 grid as proj4js holds it: lower-left corner,
/// spacing and size in radians (longitude positive west), and the shifts.
#[derive(Clone, Debug)]
pub struct Subgrid {
    pub ll: [f64; 2],
    pub del: [f64; 2],
    pub lim: [usize; 2],
    pub cvs: Vec<[f64; 2]>,
}

fn grids() -> &'static RwLock<HashMap<String, Vec<Subgrid>>> {
    static GRIDS: OnceLock<RwLock<HashMap<String, Vec<Subgrid>>>> = OnceLock::new();
    GRIDS.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Registers a loaded grid under `key` (`proj4.nadgrid(key, …)`).
pub fn put_grid(key: &str, subgrids: Vec<Subgrid>) {
    grids().write().expect("grids").insert(key.to_string(), subgrids);
}

/// Whether a grid is registered under `key`.
pub fn has_grid(key: &str) -> bool {
    grids().read().expect("grids").contains_key(key)
}

#[derive(Clone, Debug)]
enum Projection {
    LongLat,
    Etmerc {
        cgb: [f64; 6],
        cbg: [f64; 6],
        utg: [f64; 6],
        gtu: [f64; 6],
        qn: f64,
        zb: f64,
    },
    Cass {
        e0: f64,
        e1: f64,
        e2: f64,
        e3: f64,
        ml0: f64,
    },
}

/// A coordinate reference system as proj4js sets one up from its definition.
#[derive(Clone, Debug)]
pub struct Proj {
    projection: Projection,
    datum_code_wgs84: bool,
    a: f64,
    es: f64,
    e: f64,
    x0: f64,
    y0: f64,
    long0: f64,
    datum: Datum,
}

/// How a plane's datum is tied to WGS 84.
#[derive(Clone, Debug, PartialEq)]
pub enum Shift {
    /// `+towgs84=` seven parameters (position vector, arc seconds, ppm).
    Helmert([f64; 7]),
    /// `+nadgrids=` a list of grid keys, `@` marking an optional one.
    Grids(String),
}

/// What a definition states — the parts of a proj string the core writes.
#[derive(Clone, Debug, PartialEq)]
pub enum Def {
    /// `EPSG:4326`: longitude/latitude on WGS 84.
    Wgs84,
    /// `+proj=utm +zone= [+south] +datum=WGS84`.
    Utm { zone: i32, south: bool },
    /// `+proj=tmerc +lat_0=0 +lon_0= +k=1 +x_0= +y_0=0 +ellps= <shift>`.
    Tmerc { lon0: f64, x0: f64, ellps: Ellps, shift: Shift },
    /// `+proj=cass +lat_0= +lon_0= +x_0= +y_0= +ellps=bessel <shift>`.
    Cass {
        lat0: f64,
        lon0: f64,
        x0: f64,
        y0: f64,
        ellps: Ellps,
        shift: Shift,
    },
}

fn sign(x: f64) -> f64 {
    if x < 0.0 { -1.0 } else { 1.0 }
}

fn adjust_lon(x: f64) -> f64 {
    if x.abs() <= SPI { x } else { x - sign(x) * TWO_PI }
}

fn adjust_lat(x: f64) -> f64 {
    if x.abs() < HALF_PI { x } else { x - sign(x) * PI }
}

fn hypot(x: f64, y: f64) -> f64 {
    let (x, y) = (x.abs(), y.abs());
    let a = x.max(y);
    let b = x.min(y) / if a != 0.0 { a } else { 1.0 };
    a * (1.0 + b.powf(2.0)).sqrt()
}

fn log1py(x: f64) -> f64 {
    let y = 1.0 + x;
    let z = y - 1.0;
    if z == 0.0 { x } else { x * y.ln() / z }
}

fn asinhy(x: f64) -> f64 {
    let y = x.abs();
    let y = log1py(y * (1.0 + y / (hypot(1.0, y) + 1.0)));
    if x < 0.0 { -y } else { y }
}

fn sinh(x: f64) -> f64 {
    let r = x.exp();
    (r - 1.0 / r) / 2.0
}

fn cosh(x: f64) -> f64 {
    let r = x.exp();
    (r + 1.0 / r) / 2.0
}

fn gatg(pp: &[f64; 6], b: f64) -> f64 {
    let cos_2b = 2.0 * (2.0 * b).cos();
    let mut i = pp.len() - 1;
    let mut h1 = pp[i];
    let mut h2 = 0.0;
    let mut h = 0.0;
    while i > 0 {
        i -= 1;
        h = -h2 + cos_2b * h1 + pp[i];
        h2 = h1;
        h1 = h;
    }
    b + h * (2.0 * b).sin()
}

fn clens(pp: &[f64; 6], arg_r: f64) -> f64 {
    let r = 2.0 * arg_r.cos();
    let mut i = pp.len() - 1;
    let mut hr1 = pp[i];
    let mut hr2 = 0.0;
    let mut hr = 0.0;
    while i > 0 {
        i -= 1;
        hr = -hr2 + r * hr1 + pp[i];
        hr2 = hr1;
        hr1 = hr;
    }
    arg_r.sin() * hr
}

fn clens_cmplx(pp: &[f64; 6], arg_r: f64, arg_i: f64) -> (f64, f64) {
    let sin_arg_r = arg_r.sin();
    let cos_arg_r = arg_r.cos();
    let sinh_arg_i = sinh(arg_i);
    let cosh_arg_i = cosh(arg_i);
    let mut r = 2.0 * cos_arg_r * cosh_arg_i;
    let mut i = -2.0 * sin_arg_r * sinh_arg_i;
    let mut j = pp.len() - 1;
    let mut hr = pp[j];
    let mut hi1 = 0.0;
    let mut hr1 = 0.0;
    let mut hi = 0.0;
    while j > 0 {
        j -= 1;
        let hr2 = hr1;
        let hi2 = hi1;
        hr1 = hr;
        hi1 = hi;
        hr = -hr2 + r * hr1 - i * hi1 + pp[j];
        hi = -hi2 + i * hr1 + r * hi1;
    }
    r = sin_arg_r * cosh_arg_i;
    i = cos_arg_r * sinh_arg_i;
    (r * hr - i * hi, r * hi + i * hr)
}

fn e0fn(x: f64) -> f64 {
    1.0 - 0.25 * x * (1.0 + x / 16.0 * (3.0 + 1.25 * x))
}
fn e1fn(x: f64) -> f64 {
    0.375 * x * (1.0 + 0.25 * x * (1.0 + 0.46875 * x))
}
fn e2fn(x: f64) -> f64 {
    0.05859375 * x * x * (1.0 + 0.75 * x)
}
fn e3fn(x: f64) -> f64 {
    x * x * x * (35.0 / 3072.0)
}
fn mlfn(e0: f64, e1: f64, e2: f64, e3: f64, phi: f64) -> f64 {
    e0 * phi - e1 * (2.0 * phi).sin() + e2 * (4.0 * phi).sin() - e3 * (6.0 * phi).sin()
}
fn imlfn(ml: f64, e0: f64, e1: f64, e2: f64, e3: f64) -> f64 {
    let mut phi = ml / e0;
    for _ in 0..15 {
        let dphi = (ml - (e0 * phi - e1 * (2.0 * phi).sin() + e2 * (4.0 * phi).sin() - e3 * (6.0 * phi).sin()))
            / (e0 - 2.0 * e1 * (2.0 * phi).cos() + 4.0 * e2 * (4.0 * phi).cos() - 6.0 * e3 * (6.0 * phi).cos());
        phi += dphi;
        if dphi.abs() <= 0.0000000001 {
            return phi;
        }
    }
    f64::NAN
}
fn g_n(a: f64, e: f64, sinphi: f64) -> f64 {
    let temp = e * sinphi;
    a / (1.0 - temp * temp).sqrt()
}

/// `eccentricity` and `sphere` of deriveConstants for an ellipsoid.
fn ellipsoid(ellps: Ellps) -> (f64, f64, f64, f64) {
    let (a, rf) = ellps.a_rf();
    let b = (1.0 - 1.0 / rf) * a;
    let a2 = a * a;
    let b2 = b * b;
    let es = (a2 - b2) / a2;
    let e = es.sqrt();
    (a, b, es, e)
}

fn datum_of(shift: Option<&Shift>, wgs84: bool, a: f64, b: f64, es: f64) -> Datum {
    let mut kind = if wgs84 { DatumType::Wgs84 } else { DatumType::NoDatum };
    let mut params = Vec::new();
    let mut grids = Vec::new();
    if wgs84 {
        params = vec![0.0, 0.0, 0.0];
    }
    match shift {
        Some(Shift::Helmert(p)) => {
            params = p.to_vec();
            if params[0] != 0.0 || params[1] != 0.0 || params[2] != 0.0 {
                kind = DatumType::ThreeParam;
            }
            if params[3] != 0.0 || params[4] != 0.0 || params[5] != 0.0 || params[6] != 0.0 {
                kind = DatumType::SevenParam;
                params[3] *= SEC_TO_RAD;
                params[4] *= SEC_TO_RAD;
                params[5] *= SEC_TO_RAD;
                params[6] = (params[6] / 1_000_000.0) + 1.0;
            }
        }
        Some(Shift::Grids(list)) => {
            kind = DatumType::GridShift;
            for v in list.split(',') {
                if v.is_empty() {
                    continue;
                }
                let optional = v.starts_with('@');
                grids.push(GridRef {
                    name: v.trim_start_matches('@').to_string(),
                    mandatory: !optional,
                });
            }
        }
        None => {}
    }
    Datum { kind, params, grids, a, b, es }
}

fn etmerc(es: f64, k0: f64, lat0: f64) -> Projection {
    let f = es / (1.0 + (1.0 - es).sqrt());
    let n = f / (2.0 - f);
    let mut np = n;
    let mut cgb = [0.0; 6];
    let mut cbg = [0.0; 6];
    let mut utg = [0.0; 6];
    let mut gtu = [0.0; 6];
    cgb[0] = n * (2.0 + n * (-2.0 / 3.0 + n * (-2.0 + n * (116.0 / 45.0 + n * (26.0 / 45.0 + n * (-2854.0 / 675.0))))));
    cbg[0] = n * (-2.0 + n * (2.0 / 3.0 + n * (4.0 / 3.0 + n * (-82.0 / 45.0 + n * (32.0 / 45.0 + n * (4642.0 / 4725.0))))));
    np *= n;
    cgb[1] = np * (7.0 / 3.0 + n * (-8.0 / 5.0 + n * (-227.0 / 45.0 + n * (2704.0 / 315.0 + n * (2323.0 / 945.0)))));
    cbg[1] = np * (5.0 / 3.0 + n * (-16.0 / 15.0 + n * (-13.0 / 9.0 + n * (904.0 / 315.0 + n * (-1522.0 / 945.0)))));
    np *= n;
    cgb[2] = np * (56.0 / 15.0 + n * (-136.0 / 35.0 + n * (-1262.0 / 105.0 + n * (73814.0 / 2835.0))));
    cbg[2] = np * (-26.0 / 15.0 + n * (34.0 / 21.0 + n * (8.0 / 5.0 + n * (-12686.0 / 2835.0))));
    np *= n;
    cgb[3] = np * (4279.0 / 630.0 + n * (-332.0 / 35.0 + n * (-399572.0 / 14175.0)));
    cbg[3] = np * (1237.0 / 630.0 + n * (-12.0 / 5.0 + n * (-24832.0 / 14175.0)));
    np *= n;
    cgb[4] = np * (4174.0 / 315.0 + n * (-144838.0 / 6237.0));
    cbg[4] = np * (-734.0 / 315.0 + n * (109598.0 / 31185.0));
    np *= n;
    cgb[5] = np * (601676.0 / 22275.0);
    cbg[5] = np * (444337.0 / 155925.0);
    np = n.powf(2.0);
    let qn = k0 / (1.0 + n) * (1.0 + np * (1.0 / 4.0 + np * (1.0 / 64.0 + np / 256.0)));
    utg[0] = n * (-0.5 + n * (2.0 / 3.0 + n * (-37.0 / 96.0 + n * (1.0 / 360.0 + n * (81.0 / 512.0 + n * (-96199.0 / 604800.0))))));
    gtu[0] = n * (0.5 + n * (-2.0 / 3.0 + n * (5.0 / 16.0 + n * (41.0 / 180.0 + n * (-127.0 / 288.0 + n * (7891.0 / 37800.0))))));
    utg[1] = np * (-1.0 / 48.0 + n * (-1.0 / 15.0 + n * (437.0 / 1440.0 + n * (-46.0 / 105.0 + n * (1118711.0 / 3870720.0)))));
    gtu[1] = np * (13.0 / 48.0 + n * (-3.0 / 5.0 + n * (557.0 / 1440.0 + n * (281.0 / 630.0 + n * (-1983433.0 / 1935360.0)))));
    np *= n;
    utg[2] = np * (-17.0 / 480.0 + n * (37.0 / 840.0 + n * (209.0 / 4480.0 + n * (-5569.0 / 90720.0))));
    gtu[2] = np * (61.0 / 240.0 + n * (-103.0 / 140.0 + n * (15061.0 / 26880.0 + n * (167603.0 / 181440.0))));
    np *= n;
    utg[3] = np * (-4397.0 / 161280.0 + n * (11.0 / 504.0 + n * (830251.0 / 7257600.0)));
    gtu[3] = np * (49561.0 / 161280.0 + n * (-179.0 / 168.0 + n * (6601661.0 / 7257600.0)));
    np *= n;
    utg[4] = np * (-4583.0 / 161280.0 + n * (108847.0 / 3991680.0));
    gtu[4] = np * (34729.0 / 80640.0 + n * (-3418889.0 / 1995840.0));
    np *= n;
    utg[5] = np * (-20648693.0 / 638668800.0);
    gtu[5] = np * (212378941.0 / 319334400.0);
    let z = gatg(&cbg, lat0);
    let zb = -qn * (z + clens(&gtu, 2.0 * z));
    Projection::Etmerc { cgb, cbg, utg, gtu, qn, zb }
}

impl Proj {
    /// A CRS set up from a definition, as `proj4(def)` sets it up.
    pub fn new(def: &Def) -> Proj {
        match def {
            Def::Wgs84 => {
                let (a, b, es, e) = ellipsoid(Ellps::Wgs84);
                Proj {
                    projection: Projection::LongLat,
                    datum_code_wgs84: true,
                    a,
                    es,
                    e,
                    x0: 0.0,
                    y0: 0.0,
                    long0: 0.0,
                    datum: datum_of(None, true, a, b, es),
                }
            }
            Def::Utm { zone, south } => {
                let (a, b, es, e) = ellipsoid(Ellps::Wgs84);
                let long0 = ((6.0 * f64::from(zone.abs())) - 183.0) * D2R;
                let k0 = 0.9996;
                Proj {
                    projection: etmerc(es, k0, 0.0),
                    datum_code_wgs84: true,
                    a,
                    es,
                    e,
                    x0: 500_000.0,
                    y0: if *south { 10_000_000.0 } else { 0.0 },
                    long0,
                    datum: datum_of(None, true, a, b, es),
                }
            }
            Def::Tmerc { lon0, x0, ellps, shift } => {
                let (a, b, es, e) = ellipsoid(*ellps);
                Proj {
                    projection: etmerc(es, 1.0, 0.0),
                    datum_code_wgs84: false,
                    a,
                    es,
                    e,
                    x0: *x0,
                    y0: 0.0,
                    long0: lon0 * D2R,
                    datum: datum_of(Some(shift), false, a, b, es),
                }
            }
            Def::Cass {
                lat0,
                lon0,
                x0,
                y0,
                ellps,
                shift,
            } => {
                let (a, b, es, e) = ellipsoid(*ellps);
                let lat0 = lat0 * D2R;
                let (e0, e1, e2, e3) = (e0fn(es), e1fn(es), e2fn(es), e3fn(es));
                let ml0 = a * mlfn(e0, e1, e2, e3, lat0);
                Proj {
                    projection: Projection::Cass { e0, e1, e2, e3, ml0 },
                    datum_code_wgs84: false,
                    a,
                    es,
                    e,
                    x0: *x0,
                    y0: *y0,
                    long0: lon0 * D2R,
                    datum: datum_of(Some(shift), false, a, b, es),
                }
            }
        }
    }

    fn is_longlat(&self) -> bool {
        matches!(self.projection, Projection::LongLat)
    }

    fn forward(&self, x: f64, y: f64) -> (f64, f64) {
        match &self.projection {
            Projection::LongLat => (x, y),
            Projection::Etmerc { cbg, gtu, qn, zb, .. } => {
                let mut ce = adjust_lon(x - self.long0);
                let mut cn = gatg(cbg, y);
                let (sin_cn, cos_cn) = (cn.sin(), cn.cos());
                let (sin_ce, cos_ce) = (ce.sin(), ce.cos());
                cn = sin_cn.atan2(cos_ce * cos_cn);
                ce = (sin_ce * cos_cn).atan2(hypot(sin_cn, cos_cn * cos_ce));
                ce = asinhy(ce.tan());
                let tmp = clens_cmplx(gtu, 2.0 * cn, 2.0 * ce);
                cn += tmp.0;
                ce += tmp.1;
                if ce.abs() <= 2.623395162778 {
                    (self.a * (qn * ce) + self.x0, self.a * (qn * cn + zb) + self.y0)
                } else {
                    (f64::INFINITY, f64::INFINITY)
                }
            }
            Projection::Cass { e0, e1, e2, e3, ml0 } => {
                let lam = adjust_lon(x - self.long0);
                let phi = y;
                let sinphi = phi.sin();
                let cosphi = phi.cos();
                let nl = g_n(self.a, self.e, sinphi);
                let tl = phi.tan() * phi.tan();
                let al = lam * phi.cos();
                let asq = al * al;
                let cl = self.es * cosphi * cosphi / (1.0 - self.es);
                let ml = self.a * mlfn(*e0, *e1, *e2, *e3, phi);
                let x = nl * al * (1.0 - asq * tl * (1.0 / 6.0 - (8.0 - tl + 8.0 * cl) * asq / 120.0));
                let y = ml - ml0 + nl * sinphi / cosphi * asq * (0.5 + (5.0 - tl + 6.0 * cl) * asq / 24.0);
                (x + self.x0, y + self.y0)
            }
        }
    }

    fn inverse(&self, x: f64, y: f64) -> (f64, f64) {
        match &self.projection {
            Projection::LongLat => (x, y),
            Projection::Etmerc { cgb, utg, qn, zb, .. } => {
                let mut ce = (x - self.x0) * (1.0 / self.a);
                let mut cn = (y - self.y0) * (1.0 / self.a);
                cn = (cn - zb) / qn;
                ce /= qn;
                if ce.abs() <= 2.623395162778 {
                    let tmp = clens_cmplx(utg, 2.0 * cn, 2.0 * ce);
                    cn += tmp.0;
                    ce += tmp.1;
                    ce = sinh(ce).atan();
                    let (sin_cn, cos_cn) = (cn.sin(), cn.cos());
                    let (sin_ce, cos_ce) = (ce.sin(), ce.cos());
                    cn = (sin_cn * cos_ce).atan2(hypot(sin_ce, cos_ce * cos_cn));
                    ce = sin_ce.atan2(cos_ce * cos_cn);
                    (adjust_lon(ce + self.long0), gatg(cgb, cn))
                } else {
                    (f64::INFINITY, f64::INFINITY)
                }
            }
            Projection::Cass { e0, e1, e2, e3, ml0 } => {
                let px = x - self.x0;
                let py = y - self.y0;
                let x = px / self.a;
                let y = py / self.a;
                let ml1 = ml0 / self.a + y;
                let phi1 = imlfn(ml1, *e0, *e1, *e2, *e3);
                if (phi1.abs() - HALF_PI).abs() <= EPSLN {
                    return (self.long0, if y < 0.0 { -HALF_PI } else { HALF_PI });
                }
                let nl1 = g_n(self.a, self.e, phi1.sin());
                let rl1 = nl1 * nl1 * nl1 / self.a / self.a * (1.0 - self.es);
                let tl1 = phi1.tan().powf(2.0);
                let dl = x * self.a / nl1;
                let dsq = dl * dl;
                let phi = phi1 - nl1 * phi1.tan() / rl1 * dl * dl * (0.5 - (1.0 + 3.0 * tl1) * dl * dl / 24.0);
                let lam = dl * (1.0 - dsq * (tl1 / 3.0 + (1.0 + 3.0 * tl1) * tl1 * dsq / 15.0)) / phi1.cos();
                (adjust_lon(lam + self.long0), adjust_lat(phi))
            }
        }
    }
}

fn check_params(t: DatumType) -> bool {
    t == DatumType::ThreeParam || t == DatumType::SevenParam
}

fn compare_datums(s: &Datum, d: &Datum) -> bool {
    if s.kind != d.kind {
        return false;
    }
    if s.a != d.a || (s.es - d.es).abs() > 0.000000000050 {
        return false;
    }
    match s.kind {
        DatumType::ThreeParam => s.params[..3] == d.params[..3],
        DatumType::SevenParam => s.params[..7] == d.params[..7],
        _ => true,
    }
}

struct P3 {
    x: f64,
    y: f64,
    z: f64,
}

fn geodetic_to_geocentric(p: P3, es: f64, a: f64) -> P3 {
    let mut longitude = p.x;
    let mut latitude = p.y;
    let height = p.z;
    if latitude < -HALF_PI && latitude > -1.001 * HALF_PI {
        latitude = -HALF_PI;
    } else if latitude > HALF_PI && latitude < 1.001 * HALF_PI {
        latitude = HALF_PI;
    } else if latitude < -HALF_PI {
        return P3 {
            x: f64::NEG_INFINITY,
            y: f64::NEG_INFINITY,
            z: p.z,
        };
    } else if latitude > HALF_PI {
        return P3 {
            x: f64::INFINITY,
            y: f64::INFINITY,
            z: p.z,
        };
    }
    if longitude > PI {
        longitude -= 2.0 * PI;
    }
    let sin_lat = latitude.sin();
    let cos_lat = latitude.cos();
    let sin2_lat = sin_lat * sin_lat;
    let rn = a / (1.0e0 - es * sin2_lat).sqrt();
    P3 {
        x: (rn + height) * cos_lat * longitude.cos(),
        y: (rn + height) * cos_lat * longitude.sin(),
        z: ((rn * (1.0 - es)) + height) * sin_lat,
    }
}

fn geocentric_to_geodetic(p: P3, es: f64, a: f64) -> P3 {
    let genau = 1e-12;
    let genau2 = genau * genau;
    let maxiter = 30;
    let (x, y, z) = (p.x, p.y, p.z);
    let pp = (x * x + y * y).sqrt();
    let rr = (x * x + y * y + z * z).sqrt();
    let longitude;
    if pp / a < genau {
        longitude = 0.0;
        if rr / a < genau {
            return P3 { x: p.x, y: p.y, z: p.z };
        }
    } else {
        longitude = y.atan2(x);
    }
    let ct = z / rr;
    let st = pp / rr;
    let mut rx = 1.0 / (1.0 - es * (2.0 - es) * st * st).sqrt();
    let mut cphi0 = st * (1.0 - es) * rx;
    let mut sphi0 = ct * rx;
    let mut iter = 0;
    let mut height;
    let mut cphi;
    let mut sphi;
    loop {
        iter += 1;
        let rn = a / (1.0 - es * sphi0 * sphi0).sqrt();
        height = pp * cphi0 + z * sphi0 - rn * (1.0 - es * sphi0 * sphi0);
        let rk = es * rn / (rn + height);
        rx = 1.0 / (1.0 - rk * (2.0 - rk) * st * st).sqrt();
        cphi = st * (1.0 - rk) * rx;
        sphi = ct * rx;
        let sdphi = sphi * cphi0 - cphi * sphi0;
        cphi0 = cphi;
        sphi0 = sphi;
        if !(sdphi * sdphi > genau2 && iter < maxiter) {
            break;
        }
    }
    P3 {
        x: longitude,
        y: (sphi / cphi.abs()).atan(),
        z: height,
    }
}

fn geocentric_to_wgs84(p: P3, kind: DatumType, d: &[f64]) -> P3 {
    match kind {
        DatumType::ThreeParam => P3 {
            x: p.x + d[0],
            y: p.y + d[1],
            z: p.z + d[2],
        },
        _ => P3 {
            x: d[6] * (p.x - d[5] * p.y + d[4] * p.z) + d[0],
            y: d[6] * (d[5] * p.x + p.y - d[3] * p.z) + d[1],
            z: d[6] * (-d[4] * p.x + d[3] * p.y + p.z) + d[2],
        },
    }
}

fn geocentric_from_wgs84(p: P3, kind: DatumType, d: &[f64]) -> P3 {
    match kind {
        DatumType::ThreeParam => P3 {
            x: p.x - d[0],
            y: p.y - d[1],
            z: p.z - d[2],
        },
        _ => {
            let xt = (p.x - d[0]) / d[6];
            let yt = (p.y - d[1]) / d[6];
            let zt = (p.z - d[2]) / d[6];
            P3 {
                x: xt + d[5] * yt - d[4] * zt,
                y: -d[5] * xt + yt + d[3] * zt,
                z: d[4] * xt - d[3] * yt + zt,
            }
        }
    }
}

fn nad_interpolate(px: f64, py: f64, ct: &Subgrid) -> (f64, f64) {
    let tx = px / ct.del[0];
    let ty = py / ct.del[1];
    let ix = tx.floor();
    let iy = ty.floor();
    let fx = tx - ix;
    let fy = ty - iy;
    if ix < 0.0 || ix >= ct.lim[0] as f64 || iy < 0.0 || iy >= ct.lim[1] as f64 {
        return (f64::NAN, f64::NAN);
    }
    let lim0 = ct.lim[0];
    let mut inx = (iy as usize) * lim0 + (ix as usize);
    let at = |i: usize| ct.cvs.get(i).copied().unwrap_or([f64::NAN, f64::NAN]);
    let f00 = at(inx);
    inx += 1;
    let f10 = at(inx);
    inx += lim0;
    let f11 = at(inx);
    inx -= 1;
    let f01 = at(inx);
    let m11 = fx * fy;
    let m10 = fx * (1.0 - fy);
    let m00 = (1.0 - fx) * (1.0 - fy);
    let m01 = (1.0 - fx) * fy;
    (
        m00 * f00[0] + m10 * f10[0] + m01 * f01[0] + m11 * f11[0],
        m00 * f00[1] + m10 * f10[1] + m01 * f01[1] + m11 * f11[1],
    )
}

fn apply_subgrid_shift(pin: (f64, f64), inverse: bool, ct: &Subgrid) -> (f64, f64) {
    if pin.0.is_nan() {
        return (f64::NAN, f64::NAN);
    }
    let mut tb = (pin.0 - ct.ll[0], pin.1 - ct.ll[1]);
    tb.0 = adjust_lon(tb.0 - PI) + PI;
    let t = nad_interpolate(tb.0, tb.1, ct);
    if inverse {
        if t.0.is_nan() {
            return (f64::NAN, f64::NAN);
        }
        let mut t = (tb.0 - t.0, tb.1 - t.1);
        let mut i: i32 = 9;
        let tol = 1e-12;
        loop {
            let del = nad_interpolate(t.0, t.1, ct);
            if del.0.is_nan() {
                break;
            }
            let dif = (tb.0 - (del.0 + t.0), tb.1 - (del.1 + t.1));
            t.0 += dif.0;
            t.1 += dif.1;
            let go = i != 0 && dif.0.abs() > tol && dif.1.abs() > tol;
            i -= 1;
            if !go {
                break;
            }
        }
        if i < 0 {
            return (f64::NAN, f64::NAN);
        }
        (adjust_lon(t.0 + ct.ll[0]), t.1 + ct.ll[1])
    } else if !t.0.is_nan() {
        (pin.0 + t.0, pin.1 + t.1)
    } else {
        (f64::NAN, f64::NAN)
    }
}

/// `applyGridShift`: the point (radians, longitude east) shifted on the
/// datum's grids, or None where none covers it.
fn apply_grid_shift(datum: &Datum, inverse: bool, p: &mut P3) -> bool {
    if datum.grids.is_empty() {
        return false;
    }
    let input = (-p.x, p.y);
    let mut output = (f64::NAN, f64::NAN);
    let loaded = grids().read().expect("grids");
    'outer: for g in &datum.grids {
        let Some(subgrids) = loaded.get(&g.name) else {
            if g.mandatory {
                return false;
            }
            continue;
        };
        for sg in subgrids {
            let epsilon = (sg.del[1].abs() + sg.del[0].abs()) / 10000.0;
            let min_x = sg.ll[0] - epsilon;
            let min_y = sg.ll[1] - epsilon;
            let max_x = sg.ll[0] + (sg.lim[0] as f64 - 1.0) * sg.del[0] + epsilon;
            let max_y = sg.ll[1] + (sg.lim[1] as f64 - 1.0) * sg.del[1] + epsilon;
            if min_y > input.1 || min_x > input.0 || max_y < input.1 || max_x < input.0 {
                continue;
            }
            output = apply_subgrid_shift(input, inverse, sg);
            if !output.0.is_nan() {
                break 'outer;
            }
        }
    }
    if output.0.is_nan() {
        return false;
    }
    p.x = -output.0;
    p.y = output.1;
    true
}

fn datum_transform(source: &Datum, dest: &Datum, p: P3) -> Option<P3> {
    if compare_datums(source, dest) {
        return Some(p);
    }
    if source.kind == DatumType::NoDatum || dest.kind == DatumType::NoDatum {
        return Some(p);
    }
    let mut p = p;
    let mut source_a = source.a;
    let mut source_es = source.es;
    if source.kind == DatumType::GridShift {
        if !apply_grid_shift(source, false, &mut p) {
            return None;
        }
        source_a = SRS_WGS84_SEMIMAJOR;
        source_es = SRS_WGS84_ESQUARED;
    }
    let mut dest_a = dest.a;
    let mut dest_es = dest.es;
    if dest.kind == DatumType::GridShift {
        dest_a = SRS_WGS84_SEMIMAJOR;
        dest_es = SRS_WGS84_ESQUARED;
    }
    let _ = (dest.b, SRS_WGS84_SEMIMINOR);
    if source_es == dest_es && source_a == dest_a && !check_params(source.kind) && !check_params(dest.kind) {
        return Some(p);
    }
    p = geodetic_to_geocentric(p, source_es, source_a);
    if check_params(source.kind) {
        p = geocentric_to_wgs84(p, source.kind, &source.params);
    }
    if check_params(dest.kind) {
        p = geocentric_from_wgs84(p, dest.kind, &dest.params);
    }
    p = geocentric_to_geodetic(p, dest_es, dest_a);
    if dest.kind == DatumType::GridShift && !apply_grid_shift(dest, true, &mut p) {
        return None;
    }
    Some(p)
}

fn not_wgs(source: &Proj, dest: &Proj) -> bool {
    let shifted = |p: &Proj| matches!(p.datum.kind, DatumType::ThreeParam | DatumType::SevenParam | DatumType::GridShift);
    (shifted(source) && !dest.datum_code_wgs84) || (shifted(dest) && !source.datum_code_wgs84)
}

/// `proj4(from, to).forward([x, y])`: the point in `dest`, or NaNs where a
/// grid leaves it out (proj4js hands those back as it does).
pub fn transform(source: &Proj, dest: &Proj, x: f64, y: f64) -> (f64, f64) {
    if !x.is_finite() || !y.is_finite() {
        return (f64::NAN, f64::NAN);
    }
    if not_wgs(source, dest) {
        let wgs84 = Proj::new(&Def::Wgs84);
        let (x, y) = transform(source, &wgs84, x, y);
        return transform(&wgs84, dest, x, y);
    }
    let (lon, lat) = if source.is_longlat() { (x * D2R, y * D2R) } else { source.inverse(x, y) };
    let Some(p) = datum_transform(&source.datum, &dest.datum, P3 { x: lon, y: lat, z: 0.0 }) else {
        return (f64::NAN, f64::NAN);
    };
    if dest.is_longlat() { (p.x * R2D, p.y * R2D) } else { dest.forward(p.x, p.y) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utm_round_trips() {
        let wgs = Proj::new(&Def::Wgs84);
        let utm = Proj::new(&Def::Utm { zone: 32, south: false });
        let (e, n) = transform(&wgs, &utm, 9.0, 51.0);
        assert!((e - 500000.0).abs() < 1e-6, "{e}");
        let (lon, lat) = transform(&utm, &wgs, e, n);
        assert!((lon - 9.0).abs() < 1e-12 && (lat - 51.0).abs() < 1e-12);
    }
}
