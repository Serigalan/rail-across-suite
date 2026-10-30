import { SIGNAL_CATALOGUE, aspectOf, protectionSystemOf, titleIn } from '@rail-across/core/signalling'

// The words for the domain's values (schemas/signalling/1.0.schema.json), as
// PlanPro names them in German and in plain English; what the catalogue holds
// — systems, kinds, settings, aspects — in its own titles, in the language set.

const L = (de, en) => ({ de, en })

export const FUNCTION = {
  entry: L('Einfahrsignal', 'Entry signal'), exit: L('Ausfahrsignal', 'Exit signal'), entryExit: L('Ein- und Ausfahrsignal', 'Entry and exit signal'),
  intermediate: L('Zwischensignal', 'Intermediate signal'), exitIntermediate: L('Ausfahr-Zwischensignal', 'Exit intermediate signal'),
  entryBlock: L('Einfahr-Blocksignal', 'Entry block signal'), block: L('Blocksignal', 'Block signal'), groupExit: L('Gruppenausfahrsignal', 'Group exit signal'),
  groupIntermediate: L('Gruppenzwischensignal', 'Group intermediate signal'), groupExitIntermediate: L('Gruppenausfahr-/zwischensignal', 'Group exit/intermediate signal'),
  followUp: L('Nachrücksignal', 'Follow-up signal'), cover: L('Deckungssignal', 'Cover signal'), trainCover: L('Zugdeckungssignal', 'Train cover signal'),
  trainTarget: L('Zugzielsignal', 'Train target signal'), levelCrossing: L('BÜ-Überwachungssignal', 'Level crossing signal'),
  distant: L('Vorsignal, Vorsignalwiederholer', 'Distant signal, repeater'), standaloneIndicator: L('Alleinstehendes Zusatzsignal', 'Standalone indicator'),
  other: L('andere', 'other'),
}

export const VIRTUAL_FUNCTION = {
  trainStartWithoutSignal: L('Zugstart ohne Signal', 'Train start without signal'), trainTargetWithoutSignal: L('Zugziel ohne Signal', 'Train target without signal'),
  trainTargetLine: L('Zugziel Strecke', 'Train target on the line'), trainStartSiding: L('Zugstart Awanst', 'Train start siding'),
  trainTargetSiding: L('Zugziel Awanst', 'Train target siding'), trainStartMiddleSwitch: L('Zugstart Mittelweiche', 'Train start middle switch'),
  trainStartTargetBlock: L('Zugstart/-ziel Bk', 'Train start/target block'), trainStartTargetBlockZs1: L('Zugstart/-ziel Bk mit Zs 1', 'Block with Zs 1'),
  trainStartTargetBlockZs7: L('Zugstart/-ziel Bk mit Zs 7', 'Block with Zs 7'), shuntingStartTargetWithoutSignal: L('Rangierstart/-ziel ohne Signal', 'Shunting start/target without signal'),
  virtualDistant: L('Vorsignal virtuell', 'Virtual distant signal'), other: L('sonstige', 'other'),
}

export const MOUNTING = {
  mast: L('Mast', 'Mast'), bracket: L('Signalausleger', 'Bracket'), gantry: L('Signalbrücke', 'Gantry'), foundation: L('Fundament', 'Foundation'),
  special: L('Sonderkonstruktion', 'Special'), other: L('andere', 'other'),
}

export const POINT_KIND = {
  axleCounter: L('Achszählpunkt', 'Axle counter'), insulatedJointOneRail: L('Isolierstoß einschienig', 'Insulated joint, one rail'),
  insulatedJointTwoRail: L('Isolierstoß zweischienig', 'Insulated joint, two rails'), electricalJoint: L('Elektrischer Trennstoß', 'Electrical joint'),
}

export const SECTION_KIND = {
  axleCounting: L('Achszählung', 'Axle counting'), audioFrequencyCircuit: L('TF-Gleisstromkreis', 'Audio frequency circuit'),
  lowFrequencyCircuit: L('NF-Gleisstromkreis', 'Low frequency circuit'), ftgs: L('FTGS', 'FTGS'), other: L('sonstige', 'other'),
}

export const OPERATION = {
  electricRemote: L('elektrisch ferngestellt', 'electric, remote'), electricLocal: L('elektrisch ortsgestellt', 'electric, local'),
  mechanicalRemote: L('mechanisch ferngestellt', 'mechanical, remote'), mechanicalLocal: L('mechanisch ortsgestellt', 'mechanical, local'),
  notWorked: L('nicht stellbar', 'not worked'), springBack: L('Rückfallweiche', 'spring switch'), disabledLeft: L('stillgelegt links', 'disabled left'),
  disabledRight: L('stillgelegt rechts', 'disabled right'), other: L('sonstige', 'other'),
}

export const TRAIN_TYPE = {
  ZZ: L('ZZ Zug-Zugstraße', 'ZZ train to train route'), ZR: L('ZR Zug-Rangierstraße', 'ZR train to shunting route'),
  ZU: L('ZU Umfahrstraße', 'ZU bypass route'), ZH: L('ZH Hilfszugstraße', 'ZH auxiliary train route'),
  ZT: L('ZT Teilzugstraße', 'ZT partial train route'), ZTU: L('ZTU', 'ZTU'), ZUH: L('ZUH', 'ZUH'), B: L('B Blockstraße', 'B block route'),
}
export const SHUNTING_TYPE = { RR: L('RR', 'RR'), RT: L('RT', 'RT'), RTU: L('RTU', 'RTU'), RU: L('RU', 'RU') }

/** The options of a table in a language, for a SelectField. */
export const optionsOf = (table, language) => Object.entries(table).map(([value, l]) => ({ value, label: l[language] ?? l.de }))

/** The word for a value, or the value itself. */
export const word = (table, value, language) => (value == null ? '' : table[value]?.[language] ?? table[value]?.de ?? String(value))

/** The signalling systems a logical signal can be of: [{ value, label }]. */
export const signalSystemOptions = (language) =>
  Object.entries(SIGNAL_CATALOGUE.systeme).map(([value, system]) => ({ value, label: titleIn(system, language) }))

/** The kinds of signal of a system, and `other`: [{ value, label }]. */
export const signalTypeOptions = (system, language, t) => [
  ...Object.entries(SIGNAL_CATALOGUE.systeme[system]?.signalarten ?? {}).map(([value, kind]) => ({ value, label: titleIn(kind, language) })),
  { value: 'other', label: t('other') },
]

/** A logical signal in words: its kind, else its system, e.g. "Ks-Hauptsignal". */
export function logicalSignalWord(l, language) {
  const system = SIGNAL_CATALOGUE.systeme[l.system]
  return titleIn(system?.signalarten[l.type], language) ?? titleIn(system, language) ?? l.system
}

/** The train protection systems of balises and magnets: [{ value, label }]. */
export const protectionSystemOptions = (language) =>
  Object.entries(SIGNAL_CATALOGUE.zugbeeinflussung.systeme).map(([value, system]) => ({ value, label: titleIn(system, language) }))

/** The kinds of balise or magnet of a system: [{ value, label }]. */
export const baliseKindOptions = (system, language) =>
  Object.entries(protectionSystemOf(system)?.arten ?? {}).map(([value, kind]) => ({ value, label: titleIn(kind, language) }))

/** How a balise acts, in words: [{ value, label }]. */
export const effectOptions = (language) =>
  Object.entries(SIGNAL_CATALOGUE.zugbeeinflussung.wirksamkeit).map(([value, e]) => ({ value, label: titleIn(e, language) }))

/** A setting's value in words: a choice by its title, a number with its unit. */
export function settingWord(setting, value, language, t) {
  if (value === undefined) return ''
  if (setting?.typ === 'auswahl') return titleIn(setting.werte[value], language) ?? String(value)
  if (setting?.typ === 'wahrheitswert') return t(value ? 'yes' : 'no')
  return setting?.einheit ? `${value} ${setting.einheit}` : String(value)
}

/** A balise in words: its kind and the choices of its settings, e.g. "Gleismagnet · 1000-Hz-Gleismagnet". */
export function baliseWord(b, language, t) {
  const system = protectionSystemOf(b.system)
  const kind = titleIn(system?.arten[b.kind], language) ?? b.kind
  const choices = Object.entries(b.settings ?? {}).filter(([name]) => system?.einstellungen[name]?.typ === 'auswahl' && name !== 'arrangement')
    .map(([name, v]) => settingWord(system.einstellungen[name], v, language, t))
  return [kind, ...choices].join(' · ')
}

/** The setting of that name of a system — of signals or of train protection. */
function settingOf(system, name) {
  return SIGNAL_CATALOGUE.systeme[system]?.einstellungen?.[name] ?? protectionSystemOf(system)?.einstellungen?.[name]
}

/**
 * A finding of the check in words: its params, which are keys of the
 * catalogue, named by their titles in the language set — an aspect by its
 * name (Hp 0), a system, a kind, a setting by its title.
 */
export function findingText(f, t, language) {
  const p = { ...f.params }
  const aspectName = (key) => aspectOf(key)?.name ?? key
  if (p.aspect) p.aspect = aspectName(p.aspect)
  if (Array.isArray(p.aspects)) p.aspects = p.aspects.map(aspectName).join(', ')
  if (p.setting) p.setting = titleIn(settingOf(p.system, p.setting), language) ?? p.setting
  if (p.type) p.type = titleIn(SIGNAL_CATALOGUE.systeme[f.params.system]?.signalarten[p.type], language) ?? p.type
  if (p.kind) p.kind = titleIn(protectionSystemOf(f.params.system)?.arten[p.kind], language) ?? p.kind
  if (p.system) p.system = titleIn(SIGNAL_CATALOGUE.systeme[p.system] ?? protectionSystemOf(p.system), language) ?? p.system
  if (p.frame) p.frame = t(`frame_${p.frame}`)
  return t(`finding_${f.code}`, p)
}
