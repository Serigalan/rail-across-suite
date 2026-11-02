// PlanPro 1.10's enumerations and the values of the signalling domain they
// are read as (decision 0011). One table per attribute; a value a table does
// not have is read as the domain's `other` where it has one, and named in the
// import's report.

export const SIGNAL_FUNCTION = {
  Einfahr_Signal: 'entry', Ausfahr_Signal: 'exit', Einfahr_Ausfahr_Signal: 'entryExit', Zwischen_Signal: 'intermediate',
  Ausfahr_Zwischen_Signal: 'exitIntermediate', Einfahr_Block_Signal: 'entryBlock', Block_Signal: 'block',
  Gruppenausfahr_Signal: 'groupExit', Gruppenzwischen_Signal: 'groupIntermediate',
  Gruppenausfahr_Gruppenzwischen_Signal: 'groupExitIntermediate', Nachrueck_Signal: 'followUp', Deckungs_Signal: 'cover',
  Zugdeckungs_Signal: 'trainCover', Zug_Ziel_Signal: 'trainTarget', BUE_Ueberwachungssignal: 'levelCrossing',
  Vorsignal_Vorsignalwiederholer: 'distant', Alleinstehendes_Zusatzsignal: 'standaloneIndicator', andere: 'other',
}

export const VIRTUAL_FUNCTION = {
  Zug_Start_ohne_Signal: 'trainStartWithoutSignal', Zug_Ziel_ohne_Signal: 'trainTargetWithoutSignal',
  Zug_Ziel_Strecke: 'trainTargetLine', Zug_Start_Awanst: 'trainStartSiding', Zug_Ziel_Awanst: 'trainTargetSiding',
  Zug_Start_Mittelweiche: 'trainStartMiddleSwitch', Zug_Start_Ziel_Bk: 'trainStartTargetBlock',
  Zug_Start_Ziel_Bk_mit_Zs_1: 'trainStartTargetBlockZs1', Zug_Start_Ziel_Bk_mit_Zs_7: 'trainStartTargetBlockZs7',
  Rangier_Start_Ziel_ohne_Signal: 'shuntingStartTargetWithoutSignal', Vsig_virtuell: 'virtualDistant', sonstige: 'other',
}

export const SIGNAL_TYPE = {
  Hauptsignal: 'main', Hauptsperrsignal: 'mainShunting', Mehrabschnittssignal: 'multiSection',
  Mehrabschnittssperrsignal: 'multiSectionShunting', Sperrsignal: 'shunting', Vorsignal: 'distant',
  Vorsignalwiederholer: 'distantRepeater', Zugdeckungssignal: 'trainCover', andere: 'other',
}

/** PlanPro Signalsystem → a system key of the catalogue; `sonstige` is none. */
export const SIGNAL_SYSTEM = { Ks: 'ks', HV: 'hv', SV: 'sv', Hl: 'hl' }

export const MOUNTING = {
  Mast: 'mast', Signalausleger: 'bracket', Signalbruecke: 'gantry', Fundament: 'foundation', Sonderkonstruktion: 'special', andere: 'other',
}

export const FRAME = {
  Schirm: 'screen', Zusatzanzeiger: 'indicator', Blechtafel: 'plate', Keramikkoerper: 'body', Bezeichnungsschild: 'nameplate', sonstige: 'other',
}

export const DETECTION_POINT = {
  Isolierstoss_einschienig: 'insulatedJointOneRail', Isolierstoss_zweischienig: 'insulatedJointTwoRail', elektrischer_Trennstoss: 'electricalJoint',
}

export const SECTION_KIND = {
  Achszaehlanlage: 'axleCounting', TF_Gleisstromkreis: 'audioFrequencyCircuit', NF_Gleisstromkreis: 'lowFrequencyCircuit', FTGS: 'ftgs', sonstige: 'other',
}

export const OPERATION = {
  elektrisch_ferngestellt: 'electricRemote', elektrisch_ortsgestellt: 'electricLocal', mechanisch_ferngestellt: 'mechanicalRemote',
  mechanisch_ortsgestellt: 'mechanicalLocal', nicht_stellbar: 'notWorked', Rueckfallweiche: 'springBack',
  stillgelegt_links: 'disabledLeft', stillgelegt_rechts: 'disabledRight', sonstige: 'other',
}

export const POSITION = { links: 'left', rechts: 'right' }
export const VIA = { links: 'left', rechts: 'right', Spitze: 'tip' }
export const DERAILER_POSITION = { aufgelegt: 'applied', abgelegt: 'removed' }
export const TWOFOLD_KIND = { echt: 'genuine', echt_eigen: 'genuineOwn', eigen: 'own' }
export const TWOFOLD_MEASURE = { Verschluss: 'locking', Ersatzschutz: 'substitute' }

export const PZB_FREQUENCY = { '500_Hz': '500', '1000_Hz': '1000', '2000_Hz': '2000', '1000_2000_Hz': '1000+2000' }
export const PZB_EFFECT = { staendig_wirksam: 'permanent', schaltbar_von_Signal: 'switchedBySignal', sonstige: 'other' }
export const PZB_ROUTE_EFFECT = {
  staendig_wirksam_wenn_Fahrstrasse_eingestellt: 'activeWhenSet', unwirksam_wenn_Fahrstrasse_eingestellt: 'inactiveWhenSet', sonstige: 'other',
}
export const GUE_ARRANGEMENT = { '1_EA': 'startFirst', '2_AE': 'endFirst' }

export const SIDE = { links: 'left', rechts: 'right' }

/** How a GEO_Kante is shaped: the element type of the core and, for a transition, its form. */
export const GEO_FORM = {
  Gerade: { type: 0 }, Bogen: { type: 1 }, Klothoide: { type: 2, transitionType: 'clothoid' },
  Blosskurve: { type: 2, transitionType: 'bloss' }, Bloss_einfach_geschwungen: { type: 2, transitionType: 'bloss' },
  S_Form_einfach_geschwungen: { type: 2, transitionType: 'bloss' }, Uebergangsbogen_S_Form: { type: 2, transitionType: 'clothoid' },
}

/** PlanPro GEO_Koordinatensystem → EPSG: DB_REF in its Gauss-Krüger strips (CR0 … FR0, zones 2 to 5). */
export const COORDINATE_SYSTEM = { CR0: 5682, DR0: 5683, ER0: 5684, FR0: 5685 }

/** W_Kr_Art → the kind of the core's switch record for one node of it. */
export const SWITCH_KIND = { EW: 'turnout', ABW: 'turnout', IBW: 'turnout', 'ABW/IBW': 'turnout', 'IBW/ABW': 'turnout', DW: 'turnout', 'Eins. DW': 'turnout', 'Sym. ABW': 'turnout', Klothoidenweiche: 'turnout', Korbbogenweiche: 'turnout' }

/** Gleis_Abschluss_Art: which end marks are buffer stops; the rest close the planning area. */
export const BUFFER_STOPS = new Set(['Bremsprellbock', 'Festprellbock', 'Schwellenkreuz', 'Kopframpe'])
