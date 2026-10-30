// The words for the domain's values (schemas/signalling/1.0.schema.json), as
// PlanPro names them in German and in plain English.

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

export const TYPE = {
  main: L('Hauptsignal', 'Main signal'), mainShunting: L('Hauptsperrsignal', 'Main shunting signal'), multiSection: L('Mehrabschnittssignal', 'Multi-section signal'),
  multiSectionShunting: L('Mehrabschnittssperrsignal', 'Multi-section shunting signal'), shunting: L('Sperrsignal', 'Shunting signal'),
  distant: L('Vorsignal', 'Distant signal'), distantRepeater: L('Vorsignalwiederholer', 'Distant repeater'), trainCover: L('Zugdeckungssignal', 'Train cover signal'),
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

export const SYSTEM = { ks: L('Ks', 'Ks'), hv: L('H/V', 'H/V'), sv: L('Sv', 'Sv'), hl: L('Hl', 'Hl') }

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
  ZZ: L('ZZ Zug-Zugstraße', 'ZZ'), ZR: L('ZR Zug-Rangierstraße', 'ZR'), ZU: L('ZU Umfahrstraße', 'ZU'), ZH: L('ZH Hilfszugstraße', 'ZH'),
  ZT: L('ZT Teilzugstraße', 'ZT'), ZTU: L('ZTU', 'ZTU'), ZUH: L('ZUH', 'ZUH'), B: L('B Blockstraße', 'B'),
}
export const SHUNTING_TYPE = { RR: L('RR', 'RR'), RT: L('RT', 'RT'), RTU: L('RTU', 'RTU'), RU: L('RU', 'RU') }

export const FREQUENCY = { 500: L('500 Hz', '500 Hz'), 1000: L('1000 Hz', '1000 Hz'), 2000: L('2000 Hz', '2000 Hz'), '1000+2000': L('1000/2000 Hz', '1000/2000 Hz') }

/** The options of a table in a language, for a SelectField. */
export const optionsOf = (table, language) => Object.entries(table).map(([value, l]) => ({ value, label: l[language] ?? l.de }))

/** The word for a value, or the value itself. */
export const word = (table, value, language) => (value == null ? '' : table[value]?.[language] ?? table[value]?.de ?? String(value))
