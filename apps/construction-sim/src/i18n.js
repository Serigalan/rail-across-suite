import { createContext, useContext } from 'react'

// The Construction-Sim's texts, German and English. Which one is the suite's
// choice, the same for the start page and every module (@rail-across/ui).

const de = {
  tool: 'Baubetriebs-Sim',
  loading: 'Lädt …',
  no_project: 'Kein Projekt gewählt. Projekte öffnen sich auf der Seite der Baubetriebs-Sim.',
  load_failed: 'Konnte nicht laden: {error}',
  to_projects: 'Zu den Projekten',
  section_stages: 'Bauzustände',
  section_stages_intro: 'Das Netz Bauzustand für Bauzustand: der Bestand, jeder Tag, an dem etwas in oder außer Betrieb geht, der Endzustand. Die Tage stehen im Gleiseditor des Layout-Tools.',
  section_settings: 'Einstellungen',
  language: 'Sprache',
  stage: 'Bauzustand',
  stage_existing: 'Bestand',
  stage_final: 'Endzustand',
  stage_from: 'ab',
  stage_before: 'Bauzustand davor',
  stage_after: 'Bauzustand danach',
  stages_no_days: 'Noch keine Tage eingetragen: Im Gleiseditor des Layout-Tools „In Betrieb ab“ und „bis“ eines Gleises setzen.',
  stage_in_service: '{tracks} Gleise und {switches} Weichen und Verbinder in Betrieb.',
  stage_came: 'Geht in Betrieb',
  stage_went: 'Außer Betrieb',
  legend_stays: 'in Betrieb',
  legend_came: 'neu in Betrieb',
  legend_goes: 'geht danach außer Betrieb',
  legend_others: 'andere Bauzustände blass',
  revision_n: 'Revision {n}',
  note_working_copy: 'mit den Änderungen dieses Browsers',
  note_head: 'Stand des Servers',
  kind_turnout: 'Weiche',
  kind_crossing: 'Kreuzung',
  kind_single_slip: 'Kreuzungsweiche',
  kind_double_slip: 'Kreuzungsweiche',
  kind_link: 'Verbinder',
  kind_stage_link: 'Bauzustandsverbinder',
}

const en = {
  tool: 'Construction sim',
  loading: 'Loading …',
  no_project: 'No project chosen. Projects open from the construction sim\'s page.',
  load_failed: 'Could not load: {error}',
  to_projects: 'To the projects',
  section_stages: 'Construction stages',
  section_stages_intro: 'The net stage by stage: the existing state, every day something is put into or taken out of service, the final state. The days are typed in the Layout-Tool\'s track editor.',
  section_settings: 'Settings',
  language: 'Language',
  stage: 'Construction stage',
  stage_existing: 'Existing',
  stage_final: 'Final',
  stage_from: 'from',
  stage_before: 'Stage before',
  stage_after: 'Stage after',
  stages_no_days: 'No days yet: set a track\'s "in service from" and "until" in the Layout-Tool\'s track editor.',
  stage_in_service: '{tracks} tracks and {switches} switches and connectors in service.',
  stage_came: 'Comes into service',
  stage_went: 'Out of service',
  legend_stays: 'in service',
  legend_came: 'new in service',
  legend_goes: 'out of service after',
  legend_others: 'other stages faint',
  revision_n: 'revision {n}',
  note_working_copy: 'with this browser\'s changes',
  note_head: 'the server\'s state',
  kind_turnout: 'Turnout',
  kind_crossing: 'Crossing',
  kind_single_slip: 'Slip',
  kind_double_slip: 'Slip',
  kind_link: 'Connector',
  kind_stage_link: 'Stage connector',
}

const TEXTS = { de, en }

/** { t, language }: t(key, params) the text, with {name} filled from params; the key itself where there is none. */
export function textsFor(language) {
  const table = TEXTS[language] ?? TEXTS.de
  const t = (key, params) => {
    const text = table[key] ?? TEXTS.de[key] ?? key
    return params ? text.replace(/\{(\w+)\}/g, (m, k) => (params[k] != null ? String(params[k]) : m)) : text
  }
  return { t, language }
}

export const I18n = createContext(textsFor('de'))
export const useTexts = () => useContext(I18n)
