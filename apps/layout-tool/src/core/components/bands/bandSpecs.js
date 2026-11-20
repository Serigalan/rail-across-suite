import {
  trackProfile, verticalCurves, gradientAt, curvatureBand, cantBand, speedBand, bandValueAt, bandRuns, elementSpans,
} from '@rail-across/core/track'

/** The bands of a track, top to bottom. */
export const BAND_ORDER = ['curvature', 'cant', 'height', 'speed']

/** A value range padded a little, symmetric about 0 where the band is signed. */
export function bandScale(range, { signed, min }) {
  if (signed) {
    const m = Math.max(min, ...(range ?? [0, 0]).map(Math.abs)) * 1.15
    return [-m, m]
  }
  const [lo, hi] = range ?? [0, min]
  const pad = Math.max((hi - lo) * 0.12, min / 2)
  return [lo - pad, hi + pad]
}

const gradeLabel = (perMille) => {
  const v = Number(perMille.toFixed(1)) || 0
  return `${v > 0 ? '+' : ''}${v.toFixed(1)} ‰`
}

/**
 * What each band of a track draws, over the whole of it, the same for the
 * overlay and the SVG a band is given out as: its line (`points`, { s, v },
 * a null value breaks it), a thinner line under it (the height plan's tangent
 * polygon), the value range it is drawn to (`scaleOf`, given the range of the
 * values shown), the stretches that carry a label (`runs`: { from, to, text,
 * valueAt }) — the radius of an arc and the length of a transition, the cant
 * where it holds, the gradient of every stretch, the speed of each run —, and
 * whether it is signed (right curves and their cant above the zero line).
 * `t` names them.
 */
export function bandSpecs(track, t) {
  const elements = track?.elements
  const spans = elementSpans(elements)
  const length = spans.length ? spans[spans.length - 1].to : 0
  const points = track ? trackProfile(track).points ?? [] : []
  const curvature = curvatureBand(elements)
  const cant = cantBand(elements)
  const speed = speedBand(elements)
  // The built gradient: the tangent polygon, and the parabola where a
  // vertical curve rounds a change.
  const height = points.length < 2 ? [] : [...new Set([
    ...points.map(p => p.station),
    ...verticalCurves(points).flatMap(c => c.map(p => p.station)),
  ])].sort((a, b) => a - b).map(s => ({ s, v: gradientAt(points, s) }))

  const specs = {
    curvature: {
      points: curvature, signed: true, title: t('bands_curvature'), unit: '1/km', rightUp: true,
      scaleOf: (range) => bandScale(range, { signed: true, min: 0.5 }),
      runs: spans.filter(sp => sp.el.elementType === 'transition' || sp.el.radius).map(sp => ({
        from: sp.from, to: sp.to,
        text: sp.el.elementType === 'transition' ? `L ${sp.el.length.toFixed(1)}` : `R ${Math.round(Math.abs(sp.el.radius))}`,
        valueAt: (s) => bandValueAt(curvature, s),
      })),
    },
    cant: {
      points: cant, signed: true, title: t('bands_cant'), unit: 'mm', rightUp: true,
      scaleOf: (range) => bandScale(range, { signed: true, min: 20 }),
      runs: bandRuns(cant).filter(r => r.v !== 0).map(r => ({ from: r.from, to: r.to, text: `u ${Math.round(Math.abs(r.v))}`, valueAt: () => r.v })),
    },
    height: {
      points: height, under: points.map(p => ({ s: p.station, v: p.z })), title: t('bands_height'), unit: 'm',
      empty: !height.length && t('bands_no_heights'),
      scaleOf: (range) => bandScale(range, { signed: false, min: 1 }),
      runs: points.slice(1).map((b, i) => ({ a: points[i], b }))
        .filter(({ a, b }) => b.station > a.station)
        .map(({ a, b }) => ({
          from: a.station, to: b.station,
          text: gradeLabel((b.z - a.z) / (b.station - a.station) * 1000),
          valueAt: (s) => a.z + (b.z - a.z) * (s - a.station) / (b.station - a.station),
        })),
    },
    speed: {
      points: speed, title: t('bands_speed'), unit: 'km/h',
      empty: !speed.some(p => p.v != null) && t('bands_no_speed'),
      // From 0, so a step reads as the share of the speed it is.
      scaleOf: (range) => [0, Math.max(40, range?.[1] ?? 0) * 1.2],
      runs: bandRuns(speed).map(r => ({ from: r.from, to: r.to, text: `${r.v}`, valueAt: () => r.v })),
    },
  }
  return { specs, spans, length, points, bands: { curvature, cant, speed, height } }
}
