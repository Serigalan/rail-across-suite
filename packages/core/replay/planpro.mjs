// The vectors of the PlanPro import (crates/planpro): the XML reader on text
// good and broken, the plan of the import's tests and variations of it —
// curved geometry, a node that is no turnout, every kind of object the
// import reads and those it cannot place — and its signalling placed on
// another net.
import { parseXml, placeSignalling, planproRecord, readPlanPro } from '../src/planpro/index.js'
import { arcFrom, straightElement, transitionElement } from '../src/geometry/index.js'
import { W, aspect, common, frame, geoNode, id, part, place, planOf, removed, topEdge, topNode } from '../test/planproFixture.js'
import { rec } from './record.mjs'

const thrown = (op, fn, ...args) => {
  try { return rec(op, fn, ...args) } catch (err) {
    return { op, input: JSON.parse(JSON.stringify(args)), error: { message: err.message } }
  }
}

const XML = [
  '<?xml version="1.0"?><!-- c --><a x="1 &amp; 2"><b>t&lt;u&#x41;</b><c/><![CDATA[<raw>]]></a>',
  '<a><b></a>', '<a>', '', 'text only', '<a x=\'1\' y = "2" z="3" x="4" flag checked=yes>&#65;&#x1F600;&bogus;&;&#;&#x;</a>',
  '<!DOCTYPE a><?pi x?><a><!-- <b/> --><b>  spaced\n text  </b></a>', '<a><!-- open', '<a><![CDATA[x', '<a', '</a>', '<a></a></b>',
  '<ns:a xsi:type="p:Q" xsi:nil="true"><ns:b>1</ns:b>tail</ns:a>', '<a>ä&#228;ö<b>ü</b></a>', '<a>&#99999999;</a>', '<a b="&lt;&gt;&quot;&apos;"/>',
  '<?xml version="1.0"?>\n<a>\n  <b/>\n</a>\n', '<a><b/><c></c><b>2</b></a>',
]

// A net of curves: e4 runs east from g1 as an arc, e5 a clothoid into it, e6 one
// that misses its end, e7 of a form the import knows not, e8 an arc of no radius.
const GK = { easting: 4500000, northing: 5600000, zone: 5684 }
const at = (dx, dy) => ({ ...GK, easting: GK.easting + dx, northing: GK.northing + dy })
const geoEdgeOf = (g, owner, a, b, form, length, gon, rA, rB) => `<GEO_Kante>${id(g)}<GEO_Kante_Allg>${W('GEO_Form', form)}${W('GEO_Laenge', length)}${W('GEO_Radius_A', rA)}${W('GEO_Radius_B', rB)}${W('GEO_Richtungswinkel', gon)}</GEO_Kante_Allg>${W('ID_GEO_Art', owner)}${W('ID_GEO_Knoten_A', a)}${W('ID_GEO_Knoten_B', b)}</GEO_Kante>`
function curves() {
  const s = at(0, -500)
  const arc = arcFrom(s, 90, 120, 600)
  const aEnd = { easting: arc.endNode[0], northing: arc.endNode[1], zone: 5684 }
  const cl = transitionElement(aEnd, arc.endBearing, 80, 600, null, { transitionType: 'clothoid' })
  const left = arcFrom(cl.endUtm, cl.endBearing, 60, -400)
  const lEnd = { easting: left.endNode[0], northing: left.endNode[1], zone: 5684 }
  const bl = transitionElement(lEnd, left.endBearing, 50, -400, 900, { transitionType: 'bloss' })
  const f = (x) => x.toFixed(3)
  const node = (n, p) => geoNode(n, f(p.easting), f(p.northing))
  return [
    topNode('c0', 'h0'), topNode('c1', 'h1'), topNode('c2', 'h2'), topNode('c3', 'h3'), topNode('c4', 'h4'), topNode('c5', 'h5'), topNode('c6', 'h6'),
    node('h0', s), node('h1', aEnd), node('h2', cl.endUtm), node('h3', lEnd), node('h4', bl.endUtm),
    node('h5', at(300, -300)), node('h6', at(300, -200)),
    topEdge('e4', 'c0', 'c4', 'Ende', 'Ende', '400.000'),
    geoEdgeOf('gc1', 'e4', 'h0', 'h1', 'Bogen', '120.000', '100', '600,000'),
    geoEdgeOf('gc2', 'e4', 'h1', 'h2', 'Klothoide', '80.000', (arc.endBearing / 0.9).toFixed(6), '600.000', '0'),
    geoEdgeOf('gc3', 'e4', 'h2', 'h3', 'Bogen', '60.000', (cl.endBearing / 0.9).toFixed(6), '400.000'),
    geoEdgeOf('gc4', 'e4', 'h3', 'h4', 'Blosskurve', '50.000', (left.endBearing / 0.9).toFixed(6), '400.000', '900.000'),
    topEdge('e6', 'c5', 'c6', 'Ende', 'Ende', null),
    geoEdgeOf('gc6', 'e6', 'h5', 'h6', 'Klothoide', '120.000', '0', '300.000', '200.000'),
    topEdge('e7', 'c6', 'c5', 'Ende', 'Ende', '50.000'),
    geoEdgeOf('gc7', 'e7', 'h6', 'h5', 'Wendelinie', '100.000', '200', null, null),
    topEdge('e8', 'c4', 'c2', 'Ende', 'Ende', '1.000'),
    geoEdgeOf('gc8', 'e8', 'h4', 'h2', 'Bogen', '30.000', '50', null, '250'),
    // An edge of no geometry, one of a node without coordinates.
    topEdge('e9', 'c1', 'c3', 'Ende', 'Ende', '10.000'),
  ].join('')
}

// Everything the import reads that the test's plan leaves out.
const more = [
  `<Gleis_Bezeichnung>${id('gb-1')}<Bezeichnung>${W('Bez_Gleis_Bezeichnung', '1')}</Bezeichnung>${part('e1', '0', '200')}${part('e2', '0', '100')}</Gleis_Bezeichnung>`,
  `<Gleis_Bezeichnung>${id('gb-2')}<Bezeichnung>${W('Bez_Gleis_Bezeichnung', '2')}</Bezeichnung>${part('e2', '0', '100')}${part('e3', '0', '90')}</Gleis_Bezeichnung>`,
  `<Strecke>${id('st-1')}<Bezeichnung>${W('Bezeichnung_Strecke', '6340')}</Bezeichnung>${part('e1', '0', '200')}</Strecke>`,
  // A derailer on e3, a second turnout element at no node, twofold protection.
  `<W_Kr_Gsp_Element>${id('gs-1')}<Bezeichnung>${W('Oertlicher_Elementname', 'Gs1')}</Bezeichnung><W_Kr_Gsp_Element_Allg>${W('W_Kr_Gsp_Stellart', 'Rueckfallweiche')}${W('Vorzugslage_Automatik', 'true')}</W_Kr_Gsp_Element_Allg><Gleissperre_Element>${W('Gleissperre_Vorzugslage', 'aufgelegt')}</Gleissperre_Element></W_Kr_Gsp_Element>`,
  `<W_Kr_Gsp_Komponente>${id('wk-gs')}${place('e3', '30.000', 'gegen')}${W('ID_W_Kr_Gsp_Element', 'gs-1')}</W_Kr_Gsp_Komponente>`,
  `<W_Kr_Gsp_Element>${id('gs-2')}<Gleissperre_Element/></W_Kr_Gsp_Element>`,
  `<W_Kr_Gsp_Element>${id('we-9')}<W_Kr_Gsp_Element_Allg>${W('W_Kr_Gsp_Stellart', 'Handstellung')}</W_Kr_Gsp_Element_Allg><Weiche_Element/></W_Kr_Gsp_Element>`,
  `<W_Kr_Gsp_Komponente>${id('wk-9')}${place('e2', '50.000')}${W('ID_W_Kr_Gsp_Element', 'we-9')}</W_Kr_Gsp_Komponente>`,
  `<Fla_Zwieschutz>${id('zw-1')}${W('ID_W_Element', 'we-1')}${W('Zwieschutz_Art', 'echt')}<Fla_Zwieschutz_Element>${W('Massnahme_L', 'Verschluss')}${W('Massnahme_R', 'Ersatzschutz')}${W('Nachlaufverhinderung', 'false')}</Fla_Zwieschutz_Element></Fla_Zwieschutz>`,
  // Flank protection by an element, waived, of nothing, of a requester that is none; one passed on.
  `<Fla_Schutz>${id('fla-2')}<Fla_Schutz_Anforderer>${W('ID_Anforderer_Element', 'we-1')}${W('Fahrt_Ueber', 'rechts')}</Fla_Schutz_Anforderer><Fla_Schutz_W_Gsp>${W('ID_Fla_W_Gsp_Element', 'gs-1')}${W('Fla_W_Lage', 'links')}</Fla_Schutz_W_Gsp><Fla_Schutz_Weitergabe>${W('ID_Fla_Weitergabe_L', 'FLA-1')}${W('ID_Fla_Weitergabe_R', 'fla-9')}</Fla_Schutz_Weitergabe></Fla_Schutz>`,
  `<Fla_Schutz>${id('fla-3')}<Fla_Schutz_Anforderer>${W('ID_Anforderer_Element', 'we-1')}${W('Fahrt_Ueber', 'Spitze')}</Fla_Schutz_Anforderer>${W('Fla_Verzicht', 'true')}<Fla_Schutz_Weitergabe>${W('ID_Fla_Weitergabe_R', 'fla-9')}</Fla_Schutz_Weitergabe></Fla_Schutz>`,
  `<Fla_Schutz>${id('fla-4')}<Fla_Schutz_Anforderer>${W('ID_Anforderer_Element', 'we-1')}${W('Fahrt_Ueber', 'links')}</Fla_Schutz_Anforderer></Fla_Schutz>`,
  `<Fla_Schutz>${id('fla-5')}<Fla_Schutz_Anforderer>${W('ID_Anforderer_Element', 'sig-a')}${W('Fahrt_Ueber', 'links')}</Fla_Schutz_Anforderer></Fla_Schutz>`,
  `<Fla_Freimelde_Zuordnung>${id('ffz-1')}${W('ID_Fla_Schutz', 'fla-2')}${W('ID_FMA_Anlage', 'fma-w1')}</Fla_Freimelde_Zuordnung>`,
  `<Fla_Freimelde_Zuordnung>${id('ffz-2')}${W('ID_Fla_Schutz', 'fla-2')}${W('ID_FMA_Anlage', 'fma-9')}</Fla_Freimelde_Zuordnung>`,
  // A speed check, a magnet of no assignment and one off the net.
  `<PZB_Element>${id('pzb-2')}${place('e2', '10.000', 'in')}${W('ID_PZB_Element_Zuordnung', 'pz-2')}${W('PZB_Art', '2000_Hz')}<PZB_Element_GM>${W('PZB_INA', 'true')}</PZB_Element_GM><PZB_Element_GUE>${W('Pruefgeschwindigkeit', '60')}${W('GUE_Messstrecke', '21,5')}${W('GUE_Anordnung', '2_AE')}</PZB_Element_GUE></PZB_Element>`,
  `<PZB_Element_Zuordnung>${id('pz-2')}<PZB_Element_Zuordnung_BP>${W('ID_PZB_Element_Bezugspunkt', 'sig-n1')}${W('Wirksamkeit', 'staendig_wirksam')}</PZB_Element_Zuordnung_BP><PZB_Element_Zuordnung_Fstr>${W('ID_Fstr_Zug_Rangier', 'fs-1')}${W('Wirksamkeit_Fstr', 'unwirksam_wenn_Fahrstrasse_eingestellt')}</PZB_Element_Zuordnung_Fstr><PZB_Element_Zuordnung_Fstr>${W('ID_Fstr_Zug_Rangier', 'fs-9')}</PZB_Element_Zuordnung_Fstr></PZB_Element_Zuordnung>`,
  `<PZB_Element>${id('pzb-3')}${place('e2', '12.000')}<PZB_Element_GUE/></PZB_Element>`,
  `<PZB_Element>${id('pzb-4')}${place('e99', '12.000')}</PZB_Element>`,
  // A shunting route N1 → A over the section, a middle route, one of no signals; the overlap's switch.
  `<Fstr_Fahrweg>${id('fw-r')}${part('e1', '20.000', '200.000')}${part('e2', '0.000', '80.000')}${W('ID_Start', 'SIG-N1')}${W('ID_Ziel', 'sig-a')}</Fstr_Fahrweg>`,
  `<Fstr_Zug_Rangier>${id('fs-r')}${W('ID_Fstr_Fahrweg', 'fw-r')}<Fstr_Rangier>${W('Fstr_Rangier_Art', 'RR')}${W('ID_FMA_Anlage_Rangier_Frei', 'fma-w1')}${W('ID_FMA_Anlage_Rangier_Frei', 'fma-9')}</Fstr_Rangier></Fstr_Zug_Rangier>`,
  `<Fstr_Zug_Rangier>${id('fs-m')}${W('ID_Fstr_Fahrweg', 'fw-1')}<Fstr_Mittel>${W('Fstr_Mittel_Art', 'ZUM')}</Fstr_Mittel></Fstr_Zug_Rangier>`,
  `<Fstr_Zug_Rangier>${id('fs-t')}${W('ID_Fstr_Fahrweg', 'fw-1')}<Fstr_Zug>${W('Fstr_Zug_Art', 'XX')}${W('Automatische_Einstellung', 'ZL')}${W('Fstr_Vsigabstand_Verkuerzt', 'true')}<Fstr_Zug_DWeg>${W('ID_Fstr_DWeg', 'dw-9')}</Fstr_Zug_DWeg></Fstr_Zug></Fstr_Zug_Rangier>`,
  `<Fstr_Zug_Rangier>${id('fs-9')}${W('ID_Fstr_Fahrweg', 'fw-d')}</Fstr_Zug_Rangier>`,
  `<Fstr_DWeg_W_Kr>${id('dk-1')}${W('ID_Fstr_DWeg', 'dw-1')}${W('ID_W_Kr_Gsp_Element', 'we-1')}${W('Element_Verschluss', 'true')}</Fstr_DWeg_W_Kr>`,
  `<Fstr_DWeg_W_Kr>${id('dk-2')}${W('ID_Fstr_DWeg', 'dw-1')}${W('ID_W_Kr_Gsp_Element', 'gs-1')}</Fstr_DWeg_W_Kr>`,
  `<Fstr_DWeg>${id('dw-2')}<Fstr_DWeg_Spezifisch>${W('DWeg_V', '40')}${W('Aufloesung_Verzoegerung', '30')}</Fstr_DWeg_Spezifisch>${W('ID_Fstr_Fahrweg', 'fw-1')}</Fstr_DWeg>`,
  `<Fstr_DWeg>${id('dw-3')}${W('ID_Fstr_Fahrweg', 'fw-9')}</Fstr_DWeg>`,
  // Signals: a shunting signal of no system, one of an unknown function and frame, one off the net, a fictive one in both states.
  `<Signal>${id('sig-s')}${place('e3', '5.000', 'gegen', 'links')}<Signal_Real>${W('Signal_Funktion', 'Foo')}${W('Signal_Befestigungsart', 'Sonderkonstruktion')}<Signal_Real_Aktiv_Schirm>${W('Signal_Art', 'Sperrsignal')}${W('Signalsystem', 'sonstige')}</Signal_Real_Aktiv_Schirm>${W('Signalsicht_Mindest', '50')}${W('Signalsicht_Erreichbar', '120')}</Signal_Real></Signal>`,
  frame('fs1', 'sig-s', 'Schirm'), aspect('bs1', 'fs1', 'Sh_0'), frame('fs2', 'sig-s', 'Lichtpunkt'),
  `<Signal_Signalbegriff>${id('bs2')}${W('ID_Signal_Rahmen', 'fs1')}<Signalbegriff_ID xsi:type="nsSignalbegriffe_Ril_301:Sh_1"/><Signal_Signalbegriff_Allg>${W('Beleuchtet', 'ja')}${W('Geschaltet', 'true')}</Signal_Signalbegriff_Allg></Signal_Signalbegriff>`,
  `<Signal_Signalbegriff>${id('bs3')}${W('ID_Signal_Rahmen', 'fs1')}<Signalbegriff_ID xsi:type="Kennlicht"/><Signal_Signalbegriff_Allg>${W('Beleuchtet', 'nein')}${W('Geschaltet', 'false')}</Signal_Signalbegriff_Allg></Signal_Signalbegriff>`,
  `<Signal_Signalbegriff>${id('bs4')}${W('ID_Signal_Rahmen', 'fs9')}<Signalbegriff_ID xsi:type="x:Hp_0"/></Signal_Signalbegriff>`,
  `<Signal_Signalbegriff>${id('bs5')}${W('ID_Signal_Rahmen', 'fs1')}</Signal_Signalbegriff>`,
  `<Signal>${id('sig-off')}${place('e99', '5.000')}<Signal_Real/></Signal>`,
  `<Signal>${id('sig-f')}<Bezeichnung><Bezeichnung_Lageplan_Kurz><Wert xsi:nil="true"/></Bezeichnung_Lageplan_Kurz>${W('Oertlicher_Elementname', 'F')}</Bezeichnung>${place('e2', '99.000', 'beide')}${place('e99', '1.000')}<Signal_Fiktiv>${W('Fiktives_Signal_Funktion', 'unbekannt')}</Signal_Fiktiv></Signal>`,
  `<Signal>${id('sig-hv')}${place('e1', '100.000', null, null)}<Signal_Real>${W('Signal_Funktion', 'Block_Signal')}<Signal_Real_Aktiv_Schirm>${W('Signal_Art', 'Hauptsignal')}${W('Signalsystem', 'HV')}</Signal_Real_Aktiv_Schirm></Signal_Real></Signal>`,
  // Detection off the net, a section of no points, a point of no section.
  `<FMA_Komponente>${id('dp-9')}${place('e99', '60.000')}${W('ID_FMAgrenze', 'fma-w1')}</FMA_Komponente>`,
  `<FMA_Komponente>${id('dp-4')}${place('e3', '80.000', 'in')}${W('FMA_Komponente_Art', 'Isolierstoss_einschienig')}${W('ID_FMAgrenze', 'fma-2')}${W('ID_FMAgrenze', 'fma-2')}</FMA_Komponente>`,
  `<FMA_Anlage>${id('fma-2')}<FMA_Anlage_Allg>${W('FMA_Art', 'Lichtschranke')}${W('FMA_Hilffreimeldung', 'true')}</FMA_Anlage_Allg></FMA_Anlage>`,
  `<FMA_Anlage>${id('fma-3')}</FMA_Anlage>`,
  `<Gleis_Abschluss>${id('ga3')}${place('e3', '85.000')}${W('Gleis_Abschluss_Art', 'Festprellbock')}</Gleis_Abschluss>`,
  `<Gleis_Abschluss>${id('ga1')}${place('e1', '3.000')}${W('Gleis_Abschluss_Art', 'Bahnsteig')}</Gleis_Abschluss>`,
].join('')

const header = `${W('Werkzeug_Name', 'Test')}${W('Werkzeug_Version', '2.1')}${W('Erzeugung_Zeitstempel', '2026-01-02T03:04:05')}`
const project = `<Planung_P_Allg>${W('Projekt_Nummer', 'P-42')}</Planung_P_Allg><LST_Planung_Gruppe><Planung_G_Schriftfeld>${W('Bezeichnung_Anlage', 'Testdorf')}${W('Bezeichnung_Unteranlage', 'Testdorf Ost')}</Planung_G_Schriftfeld><Planung_G_Allg>${W('PlanPro_XSD_Version', '1.10.0.3')}</Planung_G_Allg></LST_Planung_Gruppe>`

// A turnout of another kind whose trunk is the left, its branches the other way round.
const swapped = (text) => text
  .replaceAll(`${W('TOP_Anschluss_A', 'Links')}${W('TOP_Anschluss_B', 'Ende')}`, `${W('TOP_Anschluss_A', 'Rechts')}${W('TOP_Anschluss_B', 'Ende')}`)
  .replaceAll(topEdge('e2', 'k1', 'k2', 'Rechts', 'Ende', '100.000'), topEdge('e2', 'k1', 'k2', 'Links', 'Ende', '100.000'))
  .replaceAll(`${W('W_Kr_Art', 'EW')}`, `${W('W_Kr_Art', 'Kreuzungsweiche')}`)
  .replaceAll(`${W('Stammgleis', 'rechts')}`, `${W('Stammgleis', 'links')}`)

const SYSTEMS = (text) => text.replaceAll(`${W('GEO_Koordinatensystem', 'ER0')}${W('GK_X', '4500290.000')}`, `${W('GEO_Koordinatensystem', 'DR0')}${W('GK_X', '4500290.000')}`)
  .replaceAll(`${W('GEO_Koordinatensystem', 'ER0')}${W('GK_X', '4500300.000')}`, `${W('GEO_Koordinatensystem', 'XX0')}${W('GK_X', '4500300.000')}`)

export default function record() {
  const out = XML.map(x => thrown('planpro.parseXml', parseXml, x))
  const plans = [
    planOf(),
    planOf({ start: common + removed + more, target: common + more + curves(), allg: header, project }),
    planOf({ start: '', target: common + curves() }),
    planOf({ start: common, target: '' }),
    swapped(planOf()),
    planOf({ start: common + topNode('k4', 'g9') + topEdge('e5', 'k1', 'k4', 'Spitze', 'Ende', '5.000') + `<GEO_Kante>${id('ge5')}${W('ID_GEO_Art', 'e5')}${W('ID_GEO_Knoten_A', 'g1')}${W('ID_GEO_Knoten_B', 'g2')}</GEO_Kante>`, target: common }),
    SYSTEMS(planOf()),
    planOf({ start: '', target: '' }),
  ]
  out.push(thrown('planpro.readPlanPro', readPlanPro, planOf()))
  out.push(thrown('planpro.readPlanPro', readPlanPro, '<x:Other/>'))
  out.push(thrown('planpro.planproRecord', planproRecord, '<a/>'))
  out.push(thrown('planpro.planproRecord', planproRecord, '<a>'))
  for (const [k, p] of plans.entries()) {
    out.push(thrown('planpro.planproRecord', planproRecord, p, k === 0 ? { id: 'b3c4d5e6-0000-4000-8000-000000000001' } : k === 1 ? { id: 'p1', title: 'Plan' } : undefined))
  }
  // The plan's signalling placed on a net of its own.
  const { record: r } = planproRecord(plans[1], { id: 'p1' })
  const P = (x, y = 0) => ({ easting: 4500000 + x, northing: 5600000 + y, zone: 5684 })
  for (const onto of [
    { tracks: [{ id: 'main', epsg: 5684, elements: [straightElement(P(0), P(300))] }], switches: [] },
    { tracks: [{ id: 'main', epsg: 5684, elements: [straightElement(P(0), P(300))] }, { id: 'side', epsg: 5684, elements: [straightElement(P(200), P(290, 20))] }] },
    { tracks: [] },
  ]) out.push(rec('planpro.placeSignalling', placeSignalling, r.signalling, r.tracks, onto))
  return { planpro: out }
}
