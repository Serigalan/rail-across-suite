// A small plan in PlanPro 1.10 and the pieces it is built of, for the
// import's tests and its recorder (replay/planpro.mjs).

// A small plan in PlanPro 1.10, built here: edge e1 runs 200 m east into the
// tip of turnout W1 (node k1); e2 goes on straight 100 m, e3 branches to the
// north-east. Entry signal A on e1, exit signal N1 on e2, a route A → N1 with
// its overlap to a danger point on e2, an axle counted section W1 over the
// switch, the 1000 Hz magnet of A, flank protection for W1. Signal X is only
// in the start state: it is removed.

export const W = (name, v) => (v == null ? '' : `<${name}><Wert>${v}</Wert></${name}>`)
export const id = (v) => W('Identitaet', v)
export const place = (edge, abstand, wirk, side) => `<Punkt_Objekt_TOP_Kante>${W('Abstand', abstand)}${W('ID_TOP_Kante', edge)}${W('Wirkrichtung', wirk)}${W('Seitliche_Lage', side)}${W('Seitlicher_Abstand', side ? '3.100' : null)}</Punkt_Objekt_TOP_Kante>`
export const part = (edge, a, b) => `<Bereich_Objekt_Teilbereich>${W('Begrenzung_A', a)}${W('Begrenzung_B', b)}${W('ID_TOP_Kante', edge)}</Bereich_Objekt_Teilbereich>`
export const aspect = (aid, frame, type, symbol) => `<Signal_Signalbegriff>${id(aid)}${W('ID_Signal_Rahmen', frame)}<Signalbegriff_ID xsi:type="nsSignalbegriffe_Ril_301:${type}">${symbol ? `<Symbol>${symbol}</Symbol>` : ''}</Signalbegriff_ID></Signal_Signalbegriff>`
export const frame = (fid, signal, art) => `<Signal_Rahmen>${id(fid)}${W('ID_Signal', signal)}${W('Rahmen_Art', art)}</Signal_Rahmen>`
export const geoNode = (n, x, y) => `<GEO_Knoten>${id(n)}</GEO_Knoten><GEO_Punkt>${id(`p-${n}`)}<GEO_Punkt_Allg>${W('GEO_Koordinatensystem', 'ER0')}${W('GK_X', x)}${W('GK_Y', y)}</GEO_Punkt_Allg>${W('ID_GEO_Knoten', n)}</GEO_Punkt>`
export const geoEdge = (g, owner, a, b, length, gon) => `<GEO_Kante>${id(g)}<GEO_Kante_Allg>${W('GEO_Form', 'Gerade')}${W('GEO_Laenge', length)}${W('GEO_Richtungswinkel', gon)}</GEO_Kante_Allg>${W('ID_GEO_Art', owner)}${W('ID_GEO_Knoten_A', a)}${W('ID_GEO_Knoten_B', b)}</GEO_Kante>`
export const topEdge = (e, a, b, ca, cb, length) => `<TOP_Kante>${id(e)}${W('ID_TOP_Knoten_A', a)}${W('ID_TOP_Knoten_B', b)}<TOP_Kante_Allg>${W('TOP_Anschluss_A', ca)}${W('TOP_Anschluss_B', cb)}${W('TOP_Laenge', length)}</TOP_Kante_Allg></TOP_Kante>`
export const topNode = (k, g) => `<TOP_Knoten>${id(k)}${W('ID_GEO_Knoten', g)}</TOP_Knoten>`

export const e3Length = Math.hypot(90, 20).toFixed(3)
export const common = [
  topNode('k0', 'g0'), topNode('k1', 'g1'), topNode('k2', 'g2'), topNode('k3', 'g3'),
  topEdge('e1', 'k0', 'k1', 'Ende', 'Spitze', '200.000'), topEdge('e2', 'k1', 'k2', 'Rechts', 'Ende', '100.000'),
  topEdge('e3', 'k1', 'k3', 'Links', 'Ende', e3Length),
  geoNode('g0', '4500000.000', '5600000.000'), geoNode('g1', '4500200.000', '5600000.000'),
  geoNode('g2', '4500300.000', '5600000.000'), geoNode('g3', '4500290.000', '5600020.000'),
  // e2's geometry runs the other way, from its end to the switch.
  geoEdge('ge1', 'e1', 'g0', 'g1', '200.000', '100.000'), geoEdge('ge2', 'e2', 'g2', 'g1', '100.000', '300.000'),
  geoEdge('ge3', 'e3', 'g1', 'g3', e3Length, '86.000'),
  `<Gleis_Abschluss>${id('ga2')}${place('e2', '100.000', 'in')}${W('Gleis_Abschluss_Art', 'Bremsprellbock')}</Gleis_Abschluss>`,
  // A: Ks multi-section signal with Zs 3 and a name plate.
  `<Signal>${id('sig-a')}<Bezeichnung>${W('Bezeichnung_Lageplan_Kurz', 'A')}${W('Kennzahl', '60')}</Bezeichnung>${place('e1', '20.000', 'in', 'rechts')}<Signal_Real>${W('Signal_Befestigungsart', 'Mast')}${W('Signal_Funktion', 'Einfahr_Signal')}<Signal_Real_Aktiv_Schirm>${W('Signal_Art', 'Mehrabschnittssignal')}${W('Signalsystem', 'Ks')}</Signal_Real_Aktiv_Schirm>${W('Signalsicht_Soll', '400')}</Signal_Real></Signal>`,
  frame('fa1', 'sig-a', 'Schirm'), aspect('ba1', 'fa1', 'Hp_0'), aspect('ba2', 'fa1', 'Ks_1'), aspect('ba3', 'fa1', 'Ks_2'), aspect('ba4', 'fa1', 'Zs_7'),
  frame('fa2', 'sig-a', 'Zusatzanzeiger'), aspect('ba5', 'fa2', 'Zs_3', '6'), frame('fa3', 'sig-a', 'Bezeichnungsschild'),
  // N1: Ks main shunting signal.
  `<Signal>${id('sig-n1')}<Bezeichnung>${W('Bezeichnung_Lageplan_Kurz', 'N1')}</Bezeichnung>${place('e2', '80.000', 'in', 'rechts')}<Signal_Real>${W('Signal_Befestigungsart', 'Mast')}${W('Signal_Funktion', 'Ausfahr_Signal')}<Signal_Real_Aktiv_Schirm>${W('Signal_Art', 'Hauptsperrsignal')}${W('Signalsystem', 'Ks')}</Signal_Real_Aktiv_Schirm></Signal_Real></Signal>`,
  frame('fn1', 'sig-n1', 'Schirm'), aspect('bn1', 'fn1', 'Hp_0'), aspect('bn2', 'fn1', 'Ks_1'), aspect('bn3', 'fn1', 'Sh_1'),
  // The clearance marker of W1, on both branches.
  `<Signal>${id('sig-gz')}${place('e2', '40.000', 'gegen')}${place('e3', '40.000', 'gegen')}<Signal_Real>${W('Signal_Funktion', 'andere')}</Signal_Real></Signal>`,
  frame('fg1', 'sig-gz', 'Keramikkoerper'), aspect('bg1', 'fg1', 'Ra_12'),
  `<Sonstiger_Punkt>${id('sp-1')}${place('e2', '95.000')}</Sonstiger_Punkt>`, `<Markanter_Punkt>${id('mp-1')}${W('ID_Markante_Stelle', 'sp-1')}</Markanter_Punkt>`,
  // Train detection: W1 over the switch.
  `<FMA_Komponente>${id('dp-1')}<Bezeichnung>${W('Bezeichnung_Lageplan_Kurz', 'W1/G1')}</Bezeichnung>${place('e1', '150.000', 'beide', 'rechts')}${W('ID_FMAgrenze', 'fma-w1')}<FMA_Komponente_Achszaehlpunkt/></FMA_Komponente>`,
  `<FMA_Komponente>${id('dp-2')}${place('e2', '60.000', 'beide', 'rechts')}${W('ID_FMAgrenze', 'fma-w1')}</FMA_Komponente>`,
  `<FMA_Komponente>${id('dp-3')}${place('e3', '60.000', 'beide', 'links')}${W('ID_FMAgrenze', 'fma-w1')}</FMA_Komponente>`,
  `<FMA_Anlage>${id('fma-w1')}<Bezeichnung>${W('Bezeichnung_Kennbuchstabe', 'W')}</Bezeichnung><FMA_Anlage_Allg>${W('FMA_Art', 'Achszaehlanlage')}</FMA_Anlage_Allg>${W('ID_Gleis_Abschnitt', 'gab-1')}</FMA_Anlage>`,
  `<Gleis_Abschnitt>${id('gab-1')}<Bezeichnung>${W('Bezeichnung_Lageplan_Kurz', '1')}${W('Kennzahl', '60')}</Bezeichnung></Gleis_Abschnitt>`,
  // The switch element W1, its component at the tip.
  `<W_Kr_Anlage>${id('wa-1')}<W_Kr_Anlage_Allg>${W('W_Kr_Art', 'EW')}${W('W_Kr_Grundform', 'EW 54-500-1:12')}</W_Kr_Anlage_Allg></W_Kr_Anlage>`,
  `<W_Kr_Gsp_Element>${id('we-1')}<Bezeichnung>${W('Bezeichnung_Lageplan_Kurz', '1')}${W('Bezeichnung_Tabelle', '60W1')}</Bezeichnung>${W('ID_W_Kr_Anlage', 'wa-1')}<W_Kr_Gsp_Element_Allg>${W('W_Kr_Gsp_Stellart', 'elektrisch_ferngestellt')}</W_Kr_Gsp_Element_Allg><Weiche_Element>${W('Weiche_Vorzugslage', 'rechts')}${W('ID_Grenzzeichen', 'sig-gz')}</Weiche_Element></W_Kr_Gsp_Element>`,
  `<W_Kr_Gsp_Komponente>${id('wk-1')}${place('e1', '200.000')}${W('ID_W_Kr_Gsp_Element', 'we-1')}<Zungenpaar>${W('Stammgleis', 'rechts')}</Zungenpaar></W_Kr_Gsp_Komponente>`,
  // Route A → N1 and its overlap.
  `<Fstr_Fahrweg>${id('fw-1')}${part('e2', '0.000', '80.000')}${part('e1', '20.000', '200.000')}${W('ID_Start', 'sig-a')}${W('ID_Ziel', 'sig-n1')}${W('Fstr_V_Hg', '60')}</Fstr_Fahrweg>`,
  `<Fstr_Fahrweg>${id('fw-d')}${part('e2', '80.000', '95.000')}${W('ID_Start', 'sig-n1')}${W('ID_Ziel', 'mp-1')}</Fstr_Fahrweg>`,
  `<Fstr_DWeg>${id('dw-1')}<Bezeichnung>${W('Bezeichnung_Fstr_DWeg', 'D1')}</Bezeichnung><Fstr_DWeg_Allg>${W('Laenge_Soll', '15')}${W('Massgebende_Neigung', '-1.50')}</Fstr_DWeg_Allg>${W('ID_FMA_Anlage_Freimeldung', 'fma-w1')}${W('ID_Fstr_Fahrweg', 'fw-d')}${W('ID_PZB_Gefahrpunkt', 'mp-1')}</Fstr_DWeg>`,
  `<Fstr_Zug_Rangier>${id('fs-1')}<Fstr_Zug_Rangier_Allg>${W('Fstr_Reihenfolge', '0')}</Fstr_Zug_Rangier_Allg>${W('ID_Fstr_Fahrweg', 'fw-1')}<Fstr_Zug>${W('Fstr_Zug_Art', 'ZZ')}<Fstr_Zug_DWeg>${W('DWeg_Vorzug', 'true')}${W('ID_Fstr_DWeg', 'dw-1')}</Fstr_Zug_DWeg></Fstr_Zug></Fstr_Zug_Rangier>`,
  `<Fstr_Signalisierung>${id('si-1')}${W('ID_Fstr_Zug_Rangier', 'fs-1')}${W('ID_Signal_Signalbegriff', 'ba2')}</Fstr_Signalisierung>`,
  `<Fstr_Signalisierung>${id('si-2')}${W('ID_Fstr_Zug_Rangier', 'fs-1')}${W('ID_Signal_Signalbegriff', 'ba5')}</Fstr_Signalisierung>`,
  // Flank protection of W1 over its left branch, by N1; PZB at A.
  `<Fla_Schutz>${id('fla-1')}<Fla_Schutz_Anforderer>${W('ID_Anforderer_Element', 'we-1')}${W('Fahrt_Ueber', 'links')}</Fla_Schutz_Anforderer><Fla_Schutz_Signal>${W('ID_Fla_Signal', 'sig-n1')}</Fla_Schutz_Signal></Fla_Schutz>`,
  `<PZB_Element>${id('pzb-1')}${place('e1', '20.000', 'in', 'rechts')}${W('ID_PZB_Element_Zuordnung', 'pz-1')}${W('PZB_Art', '1000_Hz')}<PZB_Element_GM>${W('PZB_INA', 'false')}</PZB_Element_GM></PZB_Element>`,
  `<PZB_Element_Zuordnung>${id('pz-1')}<PZB_Element_Zuordnung_BP>${W('ID_PZB_Element_Bezugspunkt', 'sig-a')}${W('Wirksamkeit', 'schaltbar_von_Signal')}</PZB_Element_Zuordnung_BP></PZB_Element_Zuordnung>`,
  `<BUE_Anlage>${id('bue-1')}</BUE_Anlage>`,
].join('')
export const removed = `<Signal>${id('sig-x')}<Bezeichnung>${W('Bezeichnung_Lageplan_Kurz', 'X')}</Bezeichnung>${place('e3', '70.000', 'gegen', 'links')}<Signal_Fiktiv>${W('Fiktives_Signal_Funktion', 'Zug_Ziel_Strecke')}</Signal_Fiktiv></Signal>`
/** The plan of `start` and `target` state objects, with header elements `allg` and `project`. */
export const planOf = ({ start = common + removed, target = common, allg = W('Werkzeug_Name', 'Test'), project = `<Planung_P_Allg>${W('Bezeichnung_Planung_Projekt', 'Neubau ESTW Testdorf')}</Planung_P_Allg>` } = {}) => `<?xml version="1.0" encoding="UTF-8"?>
<nsPlanPro:PlanPro_Schnittstelle xmlns:nsPlanPro="http://www.plan-pro.org/modell/PlanPro/1.10.0.3" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  ${id('plan')}<PlanPro_Schnittstelle_Allg>${allg}</PlanPro_Schnittstelle_Allg>
  <LST_Planung><Fachdaten><Ausgabe_Fachdaten>
    <LST_Zustand_Start><Container>${start}</Container></LST_Zustand_Start>
    <LST_Zustand_Ziel><Container>${target}</Container></LST_Zustand_Ziel>
  </Ausgabe_Fachdaten></Fachdaten>
  <Objektmanagement><LST_Planung_Projekt>${project}</LST_Planung_Projekt></Objektmanagement></LST_Planung>
</nsPlanPro:PlanPro_Schnittstelle>`
export const plan = planOf()


